use domain::Json;
use domain::mark::MarkID;
use domain::terrain::{Terrain, TerrainID, TerrainKind};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, ModelTrait, QueryFilter, QueryOrder, sea_query::Expr,
};

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
    let mut query = Terrains::find().filter(TerrainColumn::DeletedAt.is_null());

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
///
/// SYNC: this function is not multigraph-safe; sync paths must use
/// `get_edge_by_id`.
pub async fn find_parent_edge(
    db: &DatabaseConnection,
    child_id: MarkID,
) -> StoreResult<Option<Terrain>> {
    let mut edges = get_edges(db, None, Some(child_id), Some(&TerrainKind::Child)).await?;
    Ok(edges.pop())
}

/// Insert an edge with a caller-provided `TerrainID`.
///
/// The caller provides the ID on purpose: sync requires the edge to have the
/// same identity on every device, so the originating device generates it and
/// embeds it in the `CreateEdge` op. Do not generate an ID inside this
/// function.
pub async fn insert_edge(
    db: &DatabaseConnection,
    edge_id: TerrainID,
    src: MarkID,
    dst: MarkID,
    kind: &TerrainKind,
    order: Option<&str>,
    extra: Option<Json>,
) -> StoreResult<()> {
    let active = ActiveTerrain {
        id: Set(edge_id.inner()),
        src_id: Set(src.inner()),
        dst_id: Set(dst.inner()),
        kind: Set(kind_to_string(kind)),
        order: Set(order.map(str::to_string)),
        extra: Set(extra),
        // Row clocks are managed by the sync layer via set_row_clock.
        ..Default::default()
    };
    active.insert(db).await?;
    Ok(())
}

/// Insert an edge, or revive it if the row already exists (tombstoned or
/// not) — redoing a create, or a remote create winning LWW over a local
/// tombstone, must restore the existing row rather than conflict on the
/// primary key.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_edge(
    db: &DatabaseConnection,
    edge_id: TerrainID,
    src: MarkID,
    dst: MarkID,
    kind: &TerrainKind,
    order: Option<&str>,
    extra: Option<Json>,
) -> StoreResult<()> {
    match Terrains::find_by_id(edge_id.inner()).one(db).await? {
        Some(model) => {
            let mut active: ActiveTerrain = model.into_active_model();
            active.src_id = Set(src.inner());
            active.dst_id = Set(dst.inner());
            active.kind = Set(kind_to_string(kind));
            active.order = Set(order.map(str::to_string));
            active.extra = Set(extra);
            active.deleted_at = Set(None);
            active.update(db).await?;
            Ok(())
        }
        None => insert_edge(db, edge_id, src, dst, kind, order, extra).await,
    }
}

/// Fetch a single live (non-tombstoned) edge by its stable `TerrainID`.
pub async fn get_edge_by_id(
    db: &DatabaseConnection,
    edge_id: TerrainID,
) -> StoreResult<Option<Terrain>> {
    let model = Terrains::find_by_id(edge_id.inner())
        .filter(TerrainColumn::DeletedAt.is_null())
        .one(db)
        .await?;
    Ok(model.map(Terrain::from))
}

/// Update an edge's `src` and `order` by its stable `TerrainID`.
///
/// `new_order` overwrites the stored order, including clearing it with
/// `None` (order is only meaningful for certain kinds like `Child`).
/// Clears any tombstone: when an update wins LWW over a deletion, the
/// edge is live again — otherwise the losing delete would still win
/// observably.
pub async fn update_edge_by_id(
    db: &DatabaseConnection,
    edge_id: TerrainID,
    new_src: MarkID,
    new_order: Option<&str>,
) -> StoreResult<()> {
    let model = Terrains::find_by_id(edge_id.inner())
        .one(db)
        .await?
        .ok_or(StoreError::EdgeNotFound(edge_id))?;

    let mut active: ActiveTerrain = model.into_active_model();
    active.src_id = Set(new_src.inner());
    active.order = Set(new_order.map(str::to_string));
    active.deleted_at = Set(None);
    active.update(db).await?;
    Ok(())
}

/// Tombstone an edge by its stable `TerrainID`. Soft so the row keeps
/// carrying its sync clock. Succeeds silently if the edge is already
/// gone (deletes are idempotent for sync's benefit).
pub async fn delete_edge_by_id(db: &DatabaseConnection, edge_id: TerrainID) -> StoreResult<()> {
    Terrains::update_many()
        .col_expr(
            TerrainColumn::DeletedAt,
            Expr::value(chrono::Local::now().naive_utc()),
        )
        .filter(TerrainColumn::Id.eq(edge_id.inner()))
        .exec(db)
        .await?;
    Ok(())
}

/// Insert a child edge. Generates a fresh `TerrainID` and returns it so the
/// caller can record it in ops (sync addresses edges by ID).
pub async fn insert_child_edge(
    db: &DatabaseConnection,
    parent_id: MarkID,
    child_id: MarkID,
    order: &str,
) -> StoreResult<TerrainID> {
    let edge_id = TerrainID::new();
    insert_edge(
        db,
        edge_id,
        parent_id,
        child_id,
        &TerrainKind::Child,
        Some(order),
        None,
    )
    .await?;
    Ok(edge_id)
}

/// Tombstone edges where dst_id matches.
///
/// When `kind` is `None`, deletes all edges regardless of kind.
/// When `kind` is `Some(k)`, only deletes edges of that kind.
pub async fn delete_edges_by_dst(
    db: &DatabaseConnection,
    dst_id: MarkID,
    kind: Option<&TerrainKind>,
) -> StoreResult<()> {
    let mut query = Terrains::update_many()
        .col_expr(
            TerrainColumn::DeletedAt,
            Expr::value(chrono::Local::now().naive_utc()),
        )
        .filter(TerrainColumn::DstId.eq(dst_id.inner()))
        .filter(TerrainColumn::DeletedAt.is_null());

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

