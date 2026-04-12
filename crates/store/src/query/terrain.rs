use domain::mark::MarkID;
use domain::terrain::Terrain;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, ModelTrait, QueryFilter, QueryOrder,
};

use crate::entities::{ActiveTerrain, TerrainColumn, Terrains};
use crate::error::{StoreError, StoreResult};

/// Get all child edges for a parent, ordered by fractional index ASC.
pub async fn get_child_edges(
    db: &DatabaseConnection,
    parent_id: MarkID,
) -> StoreResult<Vec<Terrain>> {
    let models = Terrains::find()
        .filter(TerrainColumn::SrcId.eq(parent_id.inner()))
        .filter(TerrainColumn::Kind.eq("child"))
        .order_by_asc(TerrainColumn::Order)
        .all(db)
        .await?;
    Ok(models.into_iter().map(Terrain::from).collect())
}

/// Find the parent (child-kind) edge pointing to child_id. None if orphan.
pub async fn find_parent_edge(
    db: &DatabaseConnection,
    child_id: MarkID,
) -> StoreResult<Option<Terrain>> {
    let model = Terrains::find()
        .filter(TerrainColumn::DstId.eq(child_id.inner()))
        .filter(TerrainColumn::Kind.eq("child"))
        .one(db)
        .await?;
    Ok(model.map(Terrain::from))
}

/// Insert a child edge.
pub async fn insert_child_edge(
    db: &DatabaseConnection,
    parent_id: MarkID,
    child_id: MarkID,
    order: &str,
) -> StoreResult<()> {
    let active = ActiveTerrain {
        src_id: Set(parent_id.inner()),
        dst_id: Set(child_id.inner()),
        kind: Set("child".to_string()),
        order: Set(Some(order.to_string())),
        extra: Set(None),
    };
    active.insert(db).await?;
    Ok(())
}

/// Delete ALL edges (any kind) where dst_id matches.
pub async fn delete_edges_by_dst(db: &DatabaseConnection, dst_id: MarkID) -> StoreResult<()> {
    Terrains::delete_many()
        .filter(TerrainColumn::DstId.eq(dst_id.inner()))
        .exec(db)
        .await?;
    Ok(())
}

/// Update the child edge pointing to `dst_id`.
/// Always sets a new fractional order; optionally re-parents to a new `src_id`.
///
/// When the parent changes, this deletes the old edge and inserts a new one
/// because the primary key is `(src_id, dst_id)` — SeaORM can't update PK fields
/// in-place (the WHERE clause would use the new value and match nothing).
pub async fn update_child_edge(
    db: &DatabaseConnection,
    dst_id: MarkID,
    new_parent: Option<MarkID>,
    new_order: &str,
) -> StoreResult<()> {
    let model = Terrains::find()
        .filter(TerrainColumn::DstId.eq(dst_id.inner()))
        .filter(TerrainColumn::Kind.eq("child"))
        .one(db)
        .await?
        .ok_or(StoreError::MarkNotFound(dst_id))?;

    match new_parent {
        Some(parent) if parent.inner() != model.src_id => {
            // Reparenting: delete old edge and insert new one
            let extra = model.extra.clone();
            model.delete(db).await?;
            let active = ActiveTerrain {
                src_id: Set(parent.inner()),
                dst_id: Set(dst_id.inner()),
                kind: Set("child".to_string()),
                order: Set(Some(new_order.to_string())),
                extra: Set(extra),
            };
            active.insert(db).await?;
        }
        _ => {
            // Same parent (or None) — just update order
            let mut active: ActiveTerrain = model.into_active_model();
            active.order = Set(Some(new_order.to_string()));
            active.update(db).await?;
        }
    }
    Ok(())
}
