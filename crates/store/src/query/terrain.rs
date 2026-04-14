use domain::mark::MarkID;
use domain::terrain::{Terrain, TerrainKind};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, ModelTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

use crate::entities::{
    ActiveTerrain, TerrainColumn, Terrains, terrain_kind_to_string as kind_to_string,
};
use crate::error::{StoreError, StoreResult};

/// Query edges with optional filters on src, dst, and kind.
///
/// Results are ordered by fractional index ASC (meaningful for Child and
/// Reading edges). Pass `None` for any filter you don't want to constrain.
pub async fn get_edges(
    db: &DatabaseConnection,
    src_id: Option<MarkID>,
    dst_id: Option<MarkID>,
    kind: Option<&TerrainKind>,
) -> StoreResult<Vec<Terrain>> {
    let mut query = Terrains::find();

    if let Some(src) = src_id {
        query = query.filter(TerrainColumn::SrcId.eq(src.inner()));
    }
    if let Some(dst) = dst_id {
        query = query.filter(TerrainColumn::DstId.eq(dst.inner()));
    }
    if let Some(k) = kind {
        query = query.filter(TerrainColumn::Kind.eq(kind_to_string(k)));
    }

    let models = query.order_by_asc(TerrainColumn::Order).all(db).await?;
    Ok(models.into_iter().map(Terrain::from).collect())
}

/// Get all child edges for a parent, ordered by fractional index ASC.
pub async fn get_child_edges(
    db: &DatabaseConnection,
    parent_id: MarkID,
) -> StoreResult<Vec<Terrain>> {
    get_edges(db, Some(parent_id), None, Some(&TerrainKind::Child)).await
}

/// Find the parent (child-kind) edge pointing to child_id. None if orphan.
///
/// Convenience wrapper — returns the first Child edge pointing at this
/// mark. Under the multigraph schema a child *can* in principle have
/// more than one parent (block embeds / synced blocks). For now we
/// assume the one-parent invariant that the editor relies on.
pub async fn find_parent_edge(
    db: &DatabaseConnection,
    child_id: MarkID,
) -> StoreResult<Option<Terrain>> {
    let mut edges = get_edges(db, None, Some(child_id), Some(&TerrainKind::Child)).await?;
    Ok(edges.pop())
}

/// Insert a child edge. Generates a fresh `id` for the edge.
pub async fn insert_child_edge(
    db: &DatabaseConnection,
    parent_id: MarkID,
    child_id: MarkID,
    order: &str,
) -> StoreResult<()> {
    let active = ActiveTerrain {
        id: Set(Uuid::new_v4()),
        src_id: Set(parent_id.inner()),
        dst_id: Set(child_id.inner()),
        kind: Set(kind_to_string(&TerrainKind::Child)),
        order: Set(Some(order.to_string())),
        extra: Set(None),
    };
    active.insert(db).await?;
    Ok(())
}

/// Delete edges where dst_id matches.
///
/// When `kind` is `None`, deletes all edges regardless of kind.
/// When `kind` is `Some(k)`, only deletes edges of that kind.
pub async fn delete_edges_by_dst(
    db: &DatabaseConnection,
    dst_id: MarkID,
    kind: Option<&TerrainKind>,
) -> StoreResult<()> {
    let mut query = Terrains::delete_many().filter(TerrainColumn::DstId.eq(dst_id.inner()));

    if let Some(k) = kind {
        query = query.filter(TerrainColumn::Kind.eq(kind_to_string(k)));
    }

    query.exec(db).await?;
    Ok(())
}

/// Update the child edge pointing to `dst_id`.
/// Always sets a new fractional order; optionally re-parents to a new `src_id`.
///
/// With the new `id`-based PK, SeaORM can update `src_id` in-place without
/// the old "delete-and-insert" dance — the primary key doesn't change.
/// We still have to pick a single edge to modify; see `find_parent_edge`'s
/// caveat about the one-parent assumption.
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

    let mut active: ActiveTerrain = model.into_active_model();
    if let Some(parent) = new_parent {
        active.src_id = Set(parent.inner());
    }
    active.order = Set(Some(new_order.to_string()));
    active.update(db).await?;
    Ok(())
}
