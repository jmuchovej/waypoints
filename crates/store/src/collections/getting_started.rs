//! The getting-started collection (`mock` feature only): a Bundle under
//! `getting-started/`, embedded at build time and ingested on first open.
//!
//! Its frontpage Landmark is the idempotency anchor: present means seeded.

use domain::{Mark, MarkID};
use uuid::Uuid;

use crate::error::{StoreError, StoreResult};

include!(concat!(env!("OUT_DIR"), "/getting_started.rs"));

const ANCHOR: Uuid = Uuid::from_u128(0x45f453a4_b942_41ed_b144_57207eb74ce5);

pub fn bundle() -> transload::Bundle {
    let mut bundle = transload::Bundle::new();
    for (path, contents) in FILES {
        bundle.insert(*path, *contents);
    }
    bundle
}

/// Seed the collection. Idempotent — skips if already applied.
pub async fn run(db: &bedrock::Database) -> StoreResult<()> {
    if db.get::<Mark>(&MarkID::from(ANCHOR)).await?.is_some() {
        tracing::debug!("seed getting-started: already applied, skipping");
        return Ok(());
    }
    let ingested = transload::ingest(db, &bundle())
        .await
        .map_err(|error| StoreError::Custom(format!("the getting-started collection: {error}")))?;
    tracing::info!(
        marks = ingested.marks,
        terrains = ingested.terrains,
        "seed getting-started: ingested"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use domain::Terrain;
    use tempfile::tempdir;

    async fn seeded() -> (tempfile::TempDir, Store) {
        let dir = tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("test.db"))
            .await
            .expect("open store");
        (dir, store)
    }

    #[tokio::test]
    async fn the_collection_seeds_once() {
        let (_dir, store) = seeded().await;
        assert!(
            store
                .db()
                .get::<Mark>(&MarkID::from(ANCHOR))
                .await
                .unwrap()
                .is_some(),
            "the anchor Landmark must be seeded"
        );
        let marks = store.db().count::<Mark>().await.unwrap();
        assert!(marks > 28, "expected the corpus, got {marks} marks");

        run(store.db()).await.expect("re-seed");
        assert_eq!(
            store.db().count::<Mark>().await.unwrap(),
            marks,
            "re-seeding must not duplicate marks"
        );
        assert!(store.db().count::<Terrain>().await.unwrap() > 0);
    }

    /// Seeded into the real engine, the collection exports as itself.
    #[tokio::test]
    async fn the_collection_round_trips_through_the_store() {
        let (_dir, store) = seeded().await;
        let committed = bundle();
        let manifest: transload::Manifest = serde_json::from_str(
            committed
                .get(transload::layout::MANIFEST)
                .expect("manifest"),
        )
        .expect("a manifest");
        let meta = transload::ExportMeta {
            app_version: manifest.app_version.clone(),
            exported_at: chrono::NaiveDateTime::parse_from_str(
                &manifest.exported_at,
                "%Y-%m-%dT%H:%M:%S%.3fZ",
            )
            .expect("a timestamp"),
            territory: manifest.territory.clone(),
        };
        let exported = transload::export(store.db(), &meta).await.expect("export");
        assert!(exported.warnings.is_empty(), "{:?}", exported.warnings);

        let paths =
            |bundle: &transload::Bundle| bundle.paths().map(str::to_string).collect::<Vec<_>>();
        assert_eq!(paths(&exported.bundle), paths(&committed));
        for path in committed
            .paths()
            .filter(|path| *path != transload::layout::MANIFEST)
        {
            assert_eq!(exported.bundle.get(path), committed.get(path), "{path}");
        }
        let again: transload::Manifest = serde_json::from_str(
            exported
                .bundle
                .get(transload::layout::MANIFEST)
                .expect("manifest"),
        )
        .expect("a manifest");
        assert_eq!(again.counts, manifest.counts);
    }
}
