use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder,
};
use tauri::State;
use uuid::Uuid;
use waypoints_domain::fractional_index;
use waypoints_domain::mark::{Content, Mark, MarkData, RichText};
use store::entities::{
    ActiveMark, ActiveTerrain, MarkColumn, Marks, TerrainColumn, Terrains,
};

use crate::mark::{MarkError, MarkResult, MarkTree};
use crate::territory::TerritoryManager;

#[tauri::command]
#[specta::specta]
pub async fn mark_list(mgr: State<'_, TerritoryManager>) -> MarkResult<Vec<Mark>> {
    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let models = Marks::find()
        .filter(MarkColumn::Mark.starts_with("landmark."))
        .filter(MarkColumn::DeletedAt.is_null())
        .all(db)
        .await?;

    tracing::info!(count = models.len(), "mark_list: found landmarks");
    Ok(models.into_iter().map(Mark::from).collect())
}

#[tauri::command]
#[specta::specta]
pub async fn mark_get(mgr: State<'_, TerritoryManager>, id: Uuid) -> MarkResult<MarkTree> {
    tracing::info!(%id, "mark_get: fetching");

    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let model = Marks::find_by_id(id)
        .filter(MarkColumn::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or(MarkError::NotFound(id))?;

    let mark = Mark::from(model);
    tracing::info!(%id, is_landmark = mark.is_landmark(), "mark_get: root mark loaded");

    let expanded = expand_children(db, mark, 0).await?;
    tracing::info!(
        %id,
        children = expanded.children.len(),
        "mark_get: expansion complete"
    );
    Ok(expanded)
}

/// Recursively expand child marks.
///
/// - **Content** children are expanded recursively (their children are fetched too).
/// - **Landmark** children are returned as leaf nodes with empty `children`.
/// - Depth is capped at 20 to guard against cycles.
async fn expand_children(
    db: &sea_orm::DatabaseConnection,
    mark: Mark,
    depth: u32,
) -> MarkResult<MarkTree> {
    const MAX_DEPTH: u32 = 20;
    let mark_id = mark.id.inner();

    if depth >= MAX_DEPTH {
        tracing::debug!(%mark_id, depth, "expand_children:leaf: depth cap");
        return Ok(MarkTree {
            mark,
            children: vec![],
        });
    }

    if depth > 0 && mark.is_landmark() {
        tracing::debug!(
            %mark_id,
            depth,
            is_landmark = mark.is_landmark(),
            "expand_children:leaf: nested landmark"
        );
        return Ok(MarkTree {
            mark,
            children: vec![],
        });
    }

    // Find child terrain edges ordered by `order`
    let terrain_models = Terrains::find()
        .filter(TerrainColumn::SrcId.eq(mark_id))
        .filter(TerrainColumn::Kind.eq("child"))
        .order_by_asc(TerrainColumn::Order)
        .all(db)
        .await?;

    tracing::debug!(
        %mark_id,
        depth,
        terrain_edges = terrain_models.len(),
        "expand_children: terrain query"
    );

    if terrain_models.is_empty() {
        return Ok(MarkTree {
            mark,
            children: vec![],
        });
    }

    // Batch-fetch all destination marks
    let dst_ids: Vec<Uuid> = terrain_models.iter().map(|t| t.dst_id).collect();
    let child_models = Marks::find()
        .filter(MarkColumn::Id.is_in(dst_ids.clone()))
        .filter(MarkColumn::DeletedAt.is_null())
        .all(db)
        .await?;

    tracing::debug!(
        %mark_id,
        depth,
        requested = dst_ids.len(),
        found = child_models.len(),
        "expand_children: child marks fetched"
    );

    if child_models.len() != dst_ids.len() {
        let found_ids: std::collections::HashSet<Uuid> =
            child_models.iter().map(|m| m.id).collect();
        let missing: Vec<_> = dst_ids
            .iter()
            .filter(|id| !found_ids.contains(id))
            .collect();
        tracing::warn!(
            %mark_id,
            ?missing,
            "expand_children: some child marks missing or soft-deleted"
        );
    }

    // Index child marks by ID for ordering
    let child_map: std::collections::HashMap<Uuid, Mark> = child_models
        .into_iter()
        .map(|m| {
            let id = m.id;
            (id, Mark::from(m))
        })
        .collect();

    // Expand in terrain order
    let mut children = Vec::with_capacity(dst_ids.len());
    for dst_id in &dst_ids {
        if let Some(child_mark) = child_map.get(dst_id) {
            tracing::debug!(
                parent = %mark_id,
                child = %dst_id,
                child_is_landmark = child_mark.is_landmark(),
                depth,
                "expand_children: expanding child"
            );
            let expanded = Box::pin(expand_children(db, child_mark.clone(), depth + 1)).await?;
            children.push(expanded);
        }
    }

    tracing::debug!(
        %mark_id,
        depth,
        children_expanded = children.len(),
        "expand_children: done"
    );

    Ok(MarkTree { mark, children })
}

// ---------------------------------------------------------------------------
// Write commands
// ---------------------------------------------------------------------------

fn parse_iso_timestamp(s: &str) -> MarkResult<chrono::NaiveDateTime> {
    // Try RFC 3339 first (e.g. "2026-03-20T14:09:53.123Z")
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.naive_utc())
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
        .map_err(|e| MarkError::InvalidTimestamp(format!("{s}: {e}")))
}

#[tauri::command]
#[specta::specta]
pub async fn mark_create(
    mgr: State<'_, TerritoryManager>,
    id: Uuid,
    parent_id: Uuid,
    content: Content,
    created_at: String,
    after_id: Option<Uuid>,
) -> MarkResult<()> {
    tracing::info!(%id, %parent_id, "mark_create");

    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let parsed_ts = parse_iso_timestamp(&created_at)?;

    // Build and insert the mark
    let mark_data = MarkData::Content(content);
    let data_json =
        serde_json::to_value(&mark_data).map_err(|e| MarkError::Serialization(e.to_string()))?;

    let active = ActiveMark {
        id: Set(id),
        mark: Set(mark_data.type_name()),
        data: Set(data_json),
        extra: Set(None),
        created_at: Set(parsed_ts),
        ..Default::default()
    };
    active.insert(db).await?;

    // Compute order for the terrain edge
    let existing_edges = Terrains::find()
        .filter(TerrainColumn::SrcId.eq(parent_id))
        .filter(TerrainColumn::Kind.eq("child"))
        .order_by_asc(TerrainColumn::Order)
        .all(db)
        .await?;

    let new_order = if let Some(after) = after_id {
        // Insert after the specified sibling
        let after_idx = existing_edges.iter().position(|e| e.dst_id == after);
        match after_idx {
            Some(idx) => {
                let after_order = existing_edges[idx].order.as_deref();
                let next_order = existing_edges.get(idx + 1).and_then(|e| e.order.as_deref());
                fractional_index::key_between(after_order, next_order)
            }
            None => fractional_index::key_between(
                existing_edges.last().and_then(|e| e.order.as_deref()),
                None,
            ),
        }
    } else if let Some(first) = existing_edges.first() {
        // Insert at beginning
        fractional_index::key_between(None, first.order.as_deref())
    } else {
        fractional_index::key_between(None, None)
    };

    let terrain = ActiveTerrain {
        src_id: Set(parent_id),
        dst_id: Set(id),
        kind: Set("child".to_string()),
        order: Set(Some(new_order)),
        extra: Set(None),
    };
    terrain.insert(db).await?;

    tracing::info!(%id, %parent_id, "mark_create: done");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn mark_update(
    mgr: State<'_, TerritoryManager>,
    id: Uuid,
    content: Content,
) -> MarkResult<()> {
    tracing::info!(%id, "mark_update");

    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let model = Marks::find_by_id(id)
        .filter(MarkColumn::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or(MarkError::NotFound(id))?;

    let mark_data = MarkData::Content(content);
    let data_json =
        serde_json::to_value(&mark_data).map_err(|e| MarkError::Serialization(e.to_string()))?;

    let mut active: ActiveMark = model.into_active_model();
    active.data = Set(data_json);
    active.update(db).await?;

    tracing::info!(%id, "mark_update: done");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn mark_delete(mgr: State<'_, TerritoryManager>, id: Uuid) -> MarkResult<()> {
    tracing::info!(%id, "mark_delete");

    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let model = Marks::find_by_id(id)
        .filter(MarkColumn::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or(MarkError::NotFound(id))?;

    // Soft-delete the mark
    let now = chrono::Local::now().naive_utc();
    let mut active: ActiveMark = model.into_active_model();
    active.deleted_at = Set(Some(now));
    active.update(db).await?;

    // Remove terrain edges pointing to this mark
    Terrains::delete_many()
        .filter(TerrainColumn::DstId.eq(id))
        .exec(db)
        .await?;

    tracing::info!(%id, "mark_delete: done");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn mark_move(
    mgr: State<'_, TerritoryManager>,
    parent_id: Uuid,
    child_id: Uuid,
    after_id: Option<Uuid>,
) -> MarkResult<()> {
    tracing::info!(%parent_id, %child_id, ?after_id, "mark_move");

    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    // Fetch current child edges in order
    let edges = Terrains::find()
        .filter(TerrainColumn::SrcId.eq(parent_id))
        .filter(TerrainColumn::Kind.eq("child"))
        .order_by_asc(TerrainColumn::Order)
        .all(db)
        .await?;

    // Compute the new order key based on the target position
    let new_order = if let Some(after) = after_id {
        let after_idx = edges.iter().position(|e| e.dst_id == after);
        match after_idx {
            Some(idx) => {
                let after_order = edges[idx].order.as_deref();
                // Find the next sibling that isn't the block being moved
                let next_order = edges[idx + 1..]
                    .iter()
                    .find(|e| e.dst_id != child_id)
                    .and_then(|e| e.order.as_deref());
                fractional_index::key_between(after_order, next_order)
            }
            None => {
                fractional_index::key_between(edges.last().and_then(|e| e.order.as_deref()), None)
            }
        }
    } else {
        // Move to the beginning — before the first sibling that isn't us
        let first_order = edges
            .iter()
            .find(|e| e.dst_id != child_id)
            .and_then(|e| e.order.as_deref());
        fractional_index::key_between(None, first_order)
    };

    // Delete ALL existing child edges pointing to this mark (handles reparenting)
    Terrains::delete_many()
        .filter(TerrainColumn::DstId.eq(child_id))
        .filter(TerrainColumn::Kind.eq("child"))
        .exec(db)
        .await?;

    let terrain = ActiveTerrain {
        src_id: Set(parent_id),
        dst_id: Set(child_id),
        kind: Set("child".to_string()),
        order: Set(Some(new_order)),
        extra: Set(None),
    };
    terrain.insert(db).await?;

    tracing::info!(%parent_id, %child_id, "mark_move: done");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn mark_update_title(
    mgr: State<'_, TerritoryManager>,
    id: Uuid,
    title: RichText,
) -> MarkResult<()> {
    tracing::info!(%id, "mark_update_title");

    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let model = Marks::find_by_id(id)
        .filter(MarkColumn::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or(MarkError::NotFound(id))?;

    // Deserialize as MarkData, update the title field
    let mut mark_data: MarkData = serde_json::from_value(model.data.clone())
        .map_err(|e| MarkError::Serialization(e.to_string()))?;

    match &mut mark_data {
        MarkData::Landmark(landmark) => {
            landmark.title = title;
        }
        _ => return Err(MarkError::NotLandmark(id)),
    }

    let data_json =
        serde_json::to_value(&mark_data).map_err(|e| MarkError::Serialization(e.to_string()))?;

    let mut active: ActiveMark = model.into_active_model();
    active.data = Set(data_json);
    active.update(db).await?;

    tracing::info!(%id, "mark_update_title: done");
    Ok(())
}
