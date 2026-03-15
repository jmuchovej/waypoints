use super::{ChartIDMap, MarkIDMap};
use crate::entities::{ActiveTerrain, Terrains};
use crate::error::StoreResult;
use sea_orm::{ActiveModelTrait, EntityTrait};
use sea_orm_migration::prelude::*;
use std::collections::HashMap;
use waypoints_domain::{
    fractional_index,
    mark::MarkID,
    terrain::{Terrain, TerrainKind},
};

pub type TerrainIDMap = HashMap<(String, String), (MarkID, MarkID)>;

/// Page keys that have content children (must match `page_defs()` keys).
const PAGE_KEYS: &[&str] = &[
    "page-welcome",
    "page-japan-trip",
    "page-tokyo-restaurants",
    "page-travel-gear",
    "page-reading-list",
    "page-cosmos-notes",
    "page-elegant-universe",
];

/// Seed terrains: connect each page's parsed content marks as children.
///
/// The convention from `seed_marks` is that content marks are keyed as
/// `"{page-key}:content:{index}"`. We iterate the mark ID map to find
/// all content children for each page and create `child` terrain edges
/// with lexicographic ordering.
pub(crate) async fn seed_terrains(
    manager: &SchemaManager<'_>,
    mark_ids: &MarkIDMap,
    _chart_ids: &ChartIDMap,
) -> StoreResult<TerrainIDMap> {
    let db = manager.get_connection();
    let mut terrain_map = TerrainIDMap::new();

    for &page_key in PAGE_KEYS {
        let Some(&page_id) = mark_ids.get(page_key) else {
            continue;
        };

        // Collect top-level content children for this page, sorted by index.
        // Keys look like "{page}:content:{i}" (top-level) or
        // "{page}:content:{i}:child:{j}" (compound children).
        let prefix = format!("{page_key}:content:");
        let mut top_level: Vec<(usize, &str, MarkID)> = mark_ids
            .iter()
            .filter_map(|(key, &id)| {
                let suffix = key.strip_prefix(&prefix)?;
                // Only match top-level (single numeric index, no ":child:")
                if suffix.contains(':') {
                    return None;
                }
                let idx: usize = suffix.parse().ok()?;
                Some((idx, key.as_str(), id))
            })
            .collect();
        top_level.sort_by_key(|(idx, _, _)| *idx);

        // Create page → top-level child terrain edges
        let orders = fractional_index::generate_n(top_level.len());
        for ((_, child_key, child_id), order) in top_level.iter().zip(orders.iter()) {
            insert_terrain(
                db,
                page_id,
                *child_id,
                order.clone(),
                &mut terrain_map,
                page_key,
                child_key,
            )
            .await?;

            // Wire compound children: look for "{child_key}:child:{j}" entries
            let nested_prefix = format!("{child_key}:child:");
            let mut nested: Vec<(usize, &str, MarkID)> = mark_ids
                .iter()
                .filter_map(|(key, &id)| {
                    let suffix = key.strip_prefix(&nested_prefix)?;
                    let idx: usize = suffix.parse().ok()?;
                    Some((idx, key.as_str(), id))
                })
                .collect();
            nested.sort_by_key(|(idx, _, _)| *idx);

            if !nested.is_empty() {
                let nested_orders = fractional_index::generate_n(nested.len());
                for ((_, nk, nid), norder) in nested.iter().zip(nested_orders.iter()) {
                    insert_terrain(
                        db,
                        *child_id,
                        *nid,
                        norder.clone(),
                        &mut terrain_map,
                        page_key,
                        nk,
                    )
                    .await?;
                }
            }
        }
    }

    Ok(terrain_map)
}

async fn insert_terrain(
    db: &sea_orm_migration::SchemaManagerConnection<'_>,
    src_id: MarkID,
    dst_id: MarkID,
    order: String,
    terrain_map: &mut TerrainIDMap,
    page_key: &str,
    child_key: &str,
) -> StoreResult<()> {
    let terrain = Terrain::builder()
        .with_src_id(src_id)
        .with_dst_id(dst_id)
        .with_kind(TerrainKind::Child)
        .with_order(order)
        .build();

    let active: ActiveTerrain = terrain.into();

    let exists = Terrains::find_by_id((src_id.inner(), dst_id.inner()))
        .one(db)
        .await?;

    if exists.is_none() {
        active.insert(db).await?;
    }

    terrain_map.insert(
        (page_key.to_string(), child_key.to_string()),
        (src_id, dst_id),
    );

    Ok(())
}
