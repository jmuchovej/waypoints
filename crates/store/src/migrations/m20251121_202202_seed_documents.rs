use super::utils::SeaDBResult;
use crate::entities::{
    ActiveMark, ActiveTerrain, ChartModel, Charts, Marks, TerrainColumn, Terrains,
    terrain_kind_to_string,
};
use crate::error::StoreError;
use domain::{
    chart::{Feature, FeatureID},
    mark::{Landmark, Mark, MarkData},
};
use importer::markdown::{MarkdownImporter, WaypointsMarkdown};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{Set, Unchanged},
    ColumnTrait, DbErr, EntityTrait, QueryFilter, TryIntoModel,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Resolve a feature name to its persisted FeatureID from a chart model.
fn resolve_feature(chart: &ChartModel, name: &str) -> Option<FeatureID> {
    let features: indexmap::IndexMap<FeatureID, Feature> =
        serde_json::from_value(chart.features.clone()).ok()?;
    features
        .into_iter()
        .find(|(_, f)| f.name == name)
        .map(|(id, _)| id)
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> SeaDBResult<()> {
        let db = manager.get_connection();

        // Pre-load all charts for reading resolution
        let charts = Charts::find().all(db).await?;

        for source in documents() {
            let result = WaypointsMarkdown::import(source);

            // Insert all marks — resolve readings on the landmark (first mark)
            for (i, mut mark) in result.marks.into_iter().enumerate() {
                // Attach readings to the landmark mark
                if i == 0 && !result.readings.is_empty() {
                    if let MarkData::Landmark(ref mut landmark) = mark.data {
                        // Find the chart model to resolve feature names → IDs
                        let chart_model = charts.iter().find(|c| {
                            c.id.parse::<domain::chart::ChartID>().ok().as_ref()
                                == Some(&landmark.chart)
                        });

                        if let Some(chart_model) = chart_model {
                            for (name, reading) in &result.readings {
                                if let Some(feature_id) = resolve_feature(chart_model, name) {
                                    landmark.readings.insert(feature_id, reading.clone());
                                }
                            }
                        }
                    }
                }

                let id = mark.id;
                let exists = Marks::find_by_id(id.inner()).one(db).await?;

                let mut active: ActiveMark = mark
                    .try_into()
                    .map_err(|e: StoreError| -> DbErr { e.into() })?;

                if exists.is_some() {
                    active.id = Unchanged(id.inner());
                    active.update(db).await?;
                } else {
                    active.insert(db).await?;
                }
            }

            // Insert all terrains. Idempotency check uses (src, dst, kind)
            // rather than the old compound PK — the new PK is a single `id`
            // UUID, so `find_by_id` can't check for duplicates. We avoid
            // re-inserting an edge of the same kind between the same pair.
            for terrain in result.terrains {
                let src = terrain.src_id;
                let dst = terrain.dst_id;
                let kind_str = terrain_kind_to_string(&terrain.kind);

                let exists = Terrains::find()
                    .filter(TerrainColumn::SrcId.eq(src.inner()))
                    .filter(TerrainColumn::DstId.eq(dst.inner()))
                    .filter(TerrainColumn::Kind.eq(&kind_str))
                    .one(db)
                    .await?;

                if exists.is_none() {
                    let active: ActiveTerrain = terrain.into();
                    active.insert(db).await?;
                }
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> SeaDBResult<()> {
        let db = manager.get_connection();
        Terrains::delete_many().exec(db).await?;
        Marks::delete_many().exec(db).await?;
        Ok(())
    }
}

/// All seed document sources, embedded at compile time.
///
/// Each entry is a markdown string with YAML frontmatter that the importer
/// can convert directly into marks and terrains.
pub(crate) fn documents() -> Vec<&'static str> {
    vec![
        // ── Pages ──────────────────────────────────────────────────
        include_str!("data/demo.md"),
        include_str!("data/japan-trip.md"),
        include_str!("data/tokyo-restaurants.md"),
        include_str!("data/travel-gear.md"),
        include_str!("data/reading-list.md"),
        include_str!("data/cosmos-notes.md"),
        include_str!("data/elegant-universe-notes.md"),
        // ── People ─────────────────────────────────────────────────
        include_str!("data/people/carlo-rovelli.md"),
        include_str!("data/people/douglas-hofstadter.md"),
        include_str!("data/people/richard-dawkins.md"),
        include_str!("data/people/melanie-mitchell.md"),
        include_str!("data/people/richard-feynman.md"),
        include_str!("data/people/thomas-kuhn.md"),
        include_str!("data/people/ernest-nagel.md"),
        include_str!("data/people/james-newman.md"),
        include_str!("data/people/brian-greene.md"),
        include_str!("data/people/yuval-noah-harari.md"),
        include_str!("data/people/roger-penrose.md"),
        include_str!("data/people/carl-sagan.md"),
        // ── Books ──────────────────────────────────────────────────
        include_str!("data/books/order-of-time.md"),
        include_str!("data/books/godel-escher-bach.md"),
        include_str!("data/books/selfish-gene.md"),
        include_str!("data/books/complexity.md"),
        include_str!("data/books/surely-joking.md"),
        include_str!("data/books/structure-revolutions.md"),
        include_str!("data/books/godels-proof.md"),
        include_str!("data/books/elegant-universe.md"),
        include_str!("data/books/sapiens.md"),
        include_str!("data/books/emperors-new-mind.md"),
        include_str!("data/books/cosmos.md"),
    ]
}
