use super::{ChartIDMap, MarkIDMap};
use crate::utils::MigrationResult;
use sea_orm::{ActiveModelTrait, EntityTrait};
use sea_orm_migration::prelude::*;
use std::collections::HashMap;
use waypoints_domain::{
    fractional_index,
    mark::MarkID,
    terrain::{Terrain, TerrainKind},
};
use waypoints_entities::prelude::{ActiveTerrain, Terrains};

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
) -> MigrationResult<TerrainIDMap> {
    let db = manager.get_connection();
    let mut terrain_map = TerrainIDMap::new();

    for &page_key in PAGE_KEYS {
        let Some(&page_id) = mark_ids.get(page_key) else {
            continue;
        };

        // Collect content children for this page, sorted by index
        let prefix = format!("{page_key}:content:");
        let mut children: Vec<(usize, &str, MarkID)> = mark_ids
            .iter()
            .filter_map(|(key, &id)| {
                let suffix = key.strip_prefix(&prefix)?;
                let idx: usize = suffix.parse().ok()?;
                Some((idx, key.as_str(), id))
            })
            .collect();
        children.sort_by_key(|(idx, _, _)| *idx);

        // Create child terrain edges with fractional index ordering
        let orders = fractional_index::generate_n(children.len());
        for ((_, child_key, child_id), order) in children.iter().zip(orders.iter()) {
            let terrain = Terrain::builder()
                .with_src_id(page_id)
                .with_dst_id(*child_id)
                .with_kind(TerrainKind::Child)
                .with_order(order.clone())
                .build();

            let active: ActiveTerrain = terrain.into();

            let exists = Terrains::find_by_id((page_id.inner(), child_id.inner()))
                .one(db)
                .await?;

            if exists.is_none() {
                active.insert(db).await?;
            }

            terrain_map.insert(
                (
                    ToString::to_string(page_key),
                    ToString::to_string(child_key),
                ),
                (page_id, *child_id),
            );
        }
    }

    Ok(terrain_map)
}
