use chrono::NaiveDateTime;
use domain::Json;
use domain::mark::MarkID;
use domain::terrain::{Terrain, TerrainID, TerrainKind};

use bedrock::{Filter, Query, ScalarValue, Update};

use crate::error::{StoreError, StoreResult};
use crate::store::Store;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Truncate a `NaiveDateTime` to millisecond precision.
fn trunc_millis(dt: NaiveDateTime) -> NaiveDateTime {
    let millis = dt.and_utc().timestamp_millis();
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|d| d.naive_utc())
        .unwrap_or(dt)
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Query edges with optional filters on src, dst, and kind.
///
/// Results are ordered by fractional index ASC (meaningful for Child and
/// Reading edges). Pass `None` for any filter you don't want to constrain.
pub async fn get_edges(
    store: &Store,
    src_id: Option<MarkID>,
    dst_id: Option<MarkID>,
    kind: Option<&TerrainKind>,
) -> StoreResult<Vec<Terrain>> {
    let mut clauses = vec![Filter::missing("deleted_at")];

    if let Some(src) = src_id {
        clauses.push(Filter::eq("src_id", src.inner().to_string()));
    }
    if let Some(dst) = dst_id {
        clauses.push(Filter::eq("dst_id", dst.inner().to_string()));
    }
    if let Some(k) = kind {
        // `kind` is stored as its flattened string via `TerrainKind::Field`.
        // Filter must match the storage representation.
        clauses.push(Filter::eq("kind", ScalarValue::Text(k.to_storage())));
    }

    store
        .db()
        .query::<Terrain>(&Query::new().filter(Filter::all(clauses)).sort_asc("order"))
        .await
        .map_err(Into::into)
}

/// Get all child edges for a parent, ordered by fractional index ASC.
pub async fn get_child_edges(store: &Store, parent_id: MarkID) -> StoreResult<Vec<Terrain>> {
    get_edges(store, Some(parent_id), None, Some(&TerrainKind::Child)).await
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
pub async fn find_parent_edge(store: &Store, child_id: MarkID) -> StoreResult<Option<Terrain>> {
    let mut edges = get_edges(store, None, Some(child_id), Some(&TerrainKind::Child)).await?;
    Ok(edges.pop())
}

/// Insert an edge with a caller-provided `TerrainID`.
///
/// The caller provides the ID on purpose: sync requires the edge to have the
/// same identity on every device, so the originating device generates it and
/// embeds it in the `CreateEdge` op. Do not generate an ID inside this
/// function.
pub async fn insert_edge(
    store: &Store,
    edge_id: TerrainID,
    src: MarkID,
    dst: MarkID,
    kind: &TerrainKind,
    order: Option<&str>,
    extra: Option<Json>,
) -> StoreResult<()> {
    let terrain = Terrain {
        id: edge_id,
        src_id: src,
        dst_id: dst,
        kind: kind.clone(),
        order: order.map(str::to_string),
        extra,
        deleted_at: None,
    };
    store.db().insert(terrain).await?;
    Ok(())
}

/// Insert an edge, or revive it if the row already exists (tombstoned or
/// not) — redoing a create, or a remote create winning LWW over a local
/// tombstone, must restore the existing row rather than conflict on the
/// primary key.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_edge(
    store: &Store,
    edge_id: TerrainID,
    src: MarkID,
    dst: MarkID,
    kind: &TerrainKind,
    order: Option<&str>,
    extra: Option<Json>,
) -> StoreResult<()> {
    match store.db().get::<Terrain>(&edge_id).await? {
        Some(existing) => {
            let updated = Terrain {
                id: existing.id,
                src_id: src,
                dst_id: dst,
                kind: kind.clone(),
                order: order.map(str::to_string),
                extra,
                deleted_at: None,
            };
            store.db().update(updated).await?;
            Ok(())
        }
        None => insert_edge(store, edge_id, src, dst, kind, order, extra).await,
    }
}

/// Fetch a single live (non-tombstoned) edge by its stable `TerrainID`.
pub async fn get_edge_by_id(store: &Store, edge_id: TerrainID) -> StoreResult<Option<Terrain>> {
    let terrain = store.db().get::<Terrain>(&edge_id).await?;
    Ok(terrain.filter(|t| t.deleted_at.is_none()))
}

/// Update an edge's `src` and `order` by its stable `TerrainID`.
///
/// `new_order` overwrites the stored order, including clearing it with
/// `None` (order is only meaningful for certain kinds like `Child`).
/// Clears any tombstone: when an update wins LWW over a deletion, the
/// edge is live again — otherwise the losing delete would still win
/// observably.
pub async fn update_edge_by_id(
    store: &Store,
    edge_id: TerrainID,
    new_src: MarkID,
    new_order: Option<&str>,
) -> StoreResult<()> {
    let terrain = store
        .db()
        .get::<Terrain>(&edge_id)
        .await?
        .ok_or(StoreError::EdgeNotFound(edge_id))?;

    let updated = Terrain {
        src_id: new_src,
        order: new_order.map(str::to_string),
        deleted_at: None,
        ..terrain
    };
    store.db().update(updated).await?;
    Ok(())
}

/// Tombstone an edge by its stable `TerrainID`. Soft so the row keeps
/// carrying its sync clock. Succeeds silently if the edge is already
/// gone (deletes are idempotent for sync's benefit).
pub async fn delete_edge_by_id(store: &Store, edge_id: TerrainID) -> StoreResult<()> {
    let Some(terrain) = store.db().get::<Terrain>(&edge_id).await? else {
        // Already absent — idempotent, no error.
        return Ok(());
    };
    let updated = Terrain {
        deleted_at: Some(trunc_millis(chrono::Local::now().naive_utc())),
        ..terrain
    };
    store.db().update(updated).await?;
    Ok(())
}

/// Insert a child edge. Generates a fresh `TerrainID` and returns it so the
/// caller can record it in ops (sync addresses edges by ID).
pub async fn insert_child_edge(
    store: &Store,
    parent_id: MarkID,
    child_id: MarkID,
    order: &str,
) -> StoreResult<TerrainID> {
    let edge_id = TerrainID::new();
    insert_edge(
        store,
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
    store: &Store,
    dst_id: MarkID,
    kind: Option<&TerrainKind>,
) -> StoreResult<()> {
    let mut clauses = vec![
        Filter::eq("dst_id", dst_id.inner().to_string()),
        Filter::missing("deleted_at"),
    ];
    if let Some(k) = kind {
        clauses.push(Filter::eq("kind", ScalarValue::Text(k.to_storage())));
    }

    let now_ms = trunc_millis(chrono::Local::now().naive_utc());
    // Use update_where to tombstone all matching edges in one go.
    // Encode the timestamp as bedrock would: TIMESTAMP_FMT text.
    let ts_text = ScalarValue::Text(now_ms.format("%Y-%m-%d %H:%M:%S%.f").to_string());
    store
        .db()
        .update_where::<Terrain>(
            &Filter::all(clauses),
            &Update::new().set("deleted_at", ts_text),
        )
        .await?;
    Ok(())
}

/// Update the child edge pointing to `dst_id`.
/// Always sets a new fractional order; optionally re-parents to a new `src_id`.
///
/// With the `id`-based PK, we can update `src_id` in-place without the old
/// "delete-and-insert" dance — the primary key doesn't change.
/// We still have to pick a single edge to modify; see `find_parent_edge`'s
/// caveat about the one-parent assumption.
pub async fn update_child_edge(
    store: &Store,
    dst_id: MarkID,
    new_parent: Option<MarkID>,
    new_order: &str,
) -> StoreResult<()> {
    let terrains = store
        .db()
        .query::<Terrain>(&Query::new().filter(Filter::all([
            Filter::eq("dst_id", dst_id.inner().to_string()),
            Filter::eq("kind", ScalarValue::Text(TerrainKind::Child.to_storage())),
            Filter::missing("deleted_at"),
        ])))
        .await?;

    let terrain = terrains
        .into_iter()
        .next()
        .ok_or(StoreError::MarkNotFound(dst_id))?;

    let updated = Terrain {
        src_id: new_parent.unwrap_or(terrain.src_id),
        order: Some(new_order.to_string()),
        ..terrain
    };
    store.db().update(updated).await?;
    Ok(())
}
