//! Demo document seeding (`mock` feature only).
//!
//! Imports the embedded markdown corpus via [`WaypointsMarkdown::import`] and
//! inserts the resulting marks + terrains through bedrock. This gives dev
//! builds a rich starting graph (pages, people, books, readings) without
//! hand-authoring documents.
//!
//! Idempotency anchor is the stable UUID embedded in `data/demo.md`'s YAML
//! frontmatter; if that mark already exists, the seed is skipped. Marks and
//! terrains are each inserted with an atomic `insert_all`, and terrains only
//! after every mark has landed (terrains reference marks by id).

use std::collections::HashMap;

use bedrock::Query;
use domain::Terrain;
use domain::{Chart, ChartID, Feature, FeatureID};
use domain::{Mark, MarkData, MarkID};
use importer::{MarkdownImporter, WaypointsMarkdown};
use indexmap::IndexMap;
use uuid::Uuid;

use crate::error::StoreResult;

/// UUID of the first seeded mark (from `data/demo.md` frontmatter). Used as the
/// idempotency anchor for the entire seed.
const SEED_ANCHOR_UUID: Uuid = Uuid::from_u128(0x45f453a4_b942_41ed_b144_57207eb74ce5);

/// Seed the demo document corpus. Idempotent — skips if already applied.
pub async fn run(db: &bedrock::Database) -> StoreResult<()> {
    let anchor = MarkID::from(SEED_ANCHOR_UUID);
    if db.get::<Mark>(&anchor).await?.is_some() {
        tracing::debug!("seed demo: already applied, skipping");
        return Ok(());
    }

    // Resolve reading feature *names* (as written in frontmatter) back to their
    // FeatureIDs, per chart.
    let chart_features: HashMap<ChartID, IndexMap<FeatureID, Feature>> = db
        .query::<Chart>(&Query::new())
        .await?
        .into_iter()
        .map(|c| (c.id, c.features))
        .collect();

    let mut marks: Vec<Mark> = Vec::new();
    let mut terrains: Vec<Terrain> = Vec::new();

    for source in documents() {
        let result = WaypointsMarkdown::import(source);
        let mut doc_marks = result.marks;

        // Attach frontmatter readings to the landmark (first) mark.
        if let Some(first) = doc_marks.first_mut()
            && !result.readings.is_empty()
            && let MarkData::Landmark(landmark) = &mut first.data
            && let Some(features) = chart_features.get(&landmark.chart)
        {
            for (name, reading) in &result.readings {
                if let Some(fid) = resolve_feature(features, name) {
                    landmark.readings.insert(fid, reading.clone());
                }
            }
        }

        marks.extend(doc_marks);
        terrains.extend(result.terrains);
    }

    let mark_count = marks.len();
    let terrain_count = terrains.len();

    // Marks first (terrains reference them by id); each batch is atomic.
    db.insert_all::<Mark>(marks).await?;
    db.insert_all::<Terrain>(terrains).await?;

    tracing::info!(
        marks = mark_count,
        terrains = terrain_count,
        "seed demo: inserted documents"
    );
    Ok(())
}

/// Find a feature by its display name within a chart's feature set.
fn resolve_feature(features: &IndexMap<FeatureID, Feature>, name: &str) -> Option<FeatureID> {
    features
        .iter()
        .find(|(_, f)| f.name == name)
        .map(|(id, _)| id.clone())
}

/// All seed document sources, embedded at compile time.
fn documents() -> Vec<&'static str> {
    vec![
        // ── Pages ──────────────────────────────────────────────────
        include_str!("data/demo.md"),
        include_str!("data/japan-trip.md"),
        include_str!("data/tokyo-restaurants.md"),
        include_str!("data/travel-gear.md"),
        include_str!("data/reading-list.md"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use tempfile::tempdir;

    /// Opening a store under the `mock` feature seeds the demo corpus, and a
    /// second seed run is a no-op. Exercised against the real turso engine.
    #[tokio::test]
    async fn demo_corpus_seeds_and_is_idempotent() {
        let dir = tempdir().expect("tempdir");
        // `Store::open` runs chart seeding then `demo::run` (mock build).
        let store = Store::open(&dir.path().join("test.db"))
            .await
            .expect("open store");

        // The idempotency anchor mark landed.
        let anchor = MarkID::from(SEED_ANCHOR_UUID);
        assert!(
            store.db().get::<Mark>(&anchor).await.unwrap().is_some(),
            "anchor mark must be seeded"
        );

        // The corpus is 28 documents; each yields a landmark plus content
        // marks, so we expect well more than 28 marks and some terrains.
        let marks = store.db().count::<Mark>().await.unwrap();
        let terrains = store.db().count::<Terrain>().await.unwrap();
        assert!(marks > 28, "expected many marks, got {marks}");
        assert!(terrains > 0, "expected terrain edges, got {terrains}");

        // Re-running the seed is a no-op (anchor probe short-circuits).
        run(store.db()).await.expect("re-seed");
        assert_eq!(
            store.db().count::<Mark>().await.unwrap(),
            marks,
            "re-seeding must not duplicate marks"
        );
    }
}
