// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The demo territory a store screenshot is captured against.
//!
//! A returning user's app-data directory, written on the host by the code the
//! app itself runs: `settings.json` with the territory registry,
//! `territories/<id>/waypoints.db`, and `sync/` holding an identity. The
//! capture places it in a release build's data directory before first launch,
//! so the build opens onto it with no onboarding.
//!
//! The identity is throwaway: minted fresh on every run, never given a
//! Station, so it can reach nothing. The store is seeded because `xtask`
//! builds `store` with `mock` (via `cartograph/development`); a release build
//! opening it seeds nothing.

use super::SettingsRegistryStore;
use anyhow::{Context as _, Result, bail};
use cartograph::{RegistryStore as _, WaypointsRuntime};
use std::{path::Path, sync::Arc};
use uuid::Uuid;

/// The registry name of the placed territory.
pub const NAME: &str = "Demo";

/// What [`write`] produced.
#[derive(Debug)]
pub struct DemoTerritory {
    pub id: Uuid,
    pub marks: u64,
}

/// Write a demo app-data directory at `dir`, which must not exist or be empty.
///
/// The `getting-started` Bundle is always ingested (the seeder runs on open);
/// `bundle` names a Bundle directory ingested on top of it.
pub async fn write(dir: &Path, bundle: Option<&Path>) -> Result<DemoTerritory> {
    if dir.exists() && dir.read_dir()?.next().is_some() {
        bail!(
            "{} is not empty; the demo territory is written fresh",
            dir.display()
        );
    }

    let registry = Arc::new(SettingsRegistryStore::new(dir.join("settings.json")));
    let rt = WaypointsRuntime::new(dir.join("territories"), dir.join("sync"), registry.clone());

    rt.sync()
        .identity_create()
        .await
        .context("minting the demo identity")?;
    let territory = rt
        .territory()
        .create(NAME.to_string())
        .await
        .context("creating the demo territory")?;
    // `close` flushes the store, so the placed database needs no WAL beside
    // it — and forgets the active territory, which is restored below.
    rt.territory().close().await?;

    let db_path = dir
        .join("territories")
        .join(territory.id.to_string())
        .join("waypoints.db");
    let store = store::open(&db_path).await?;
    if let Some(bundle) = bundle {
        let bundle = transload::Bundle::read_dir(bundle)
            .with_context(|| format!("reading the Bundle at {}", bundle.display()))?;
        transload::ingest(store.db(), &bundle)
            .await
            .context("ingesting the Bundle")?;
    }
    let marks = store.db().count::<domain::Mark>().await?;
    store::checkpoint(&store).await;
    drop(store);

    let mut entries = registry.load().await?;
    entries.last_active_id = Some(territory.id);
    registry.save(&entries).await?;

    Ok(DemoTerritory {
        id: territory.id,
        marks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cartograph::RegistryStore as _;

    async fn reopen(dir: &Path) -> (WaypointsRuntime, Arc<SettingsRegistryStore>) {
        let registry = Arc::new(SettingsRegistryStore::new(dir.join("settings.json")));
        let rt = WaypointsRuntime::new(dir.join("territories"), dir.join("sync"), registry.clone());
        rt.resume().await.expect("resume");
        (rt, registry)
    }

    #[tokio::test]
    async fn a_returning_user_opens_onto_the_seeded_territory() {
        let tmp = tempfile::tempdir().unwrap();
        let demo = write(tmp.path(), None).await.unwrap();
        assert!(
            demo.marks > 28,
            "expected the getting-started corpus, got {}",
            demo.marks
        );

        // What the app's onboarding gate reads: a territory and an identity.
        let (rt, registry) = reopen(tmp.path()).await;
        let entries = registry.load().await.unwrap();
        assert_eq!(entries.last_active_id, Some(demo.id));
        assert_eq!(entries.registry.len(), 1);
        assert!(rt.sync().status().await.unwrap().has_identity);
        assert!(!rt.sync().status().await.unwrap().station_configured);
        assert_eq!(
            rt.territory().get_active().await.unwrap().map(|t| t.id),
            Some(demo.id)
        );
    }

    #[tokio::test]
    async fn two_runs_hold_the_same_content() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let (a, b) = (
            write(a.path(), None).await.unwrap(),
            write(b.path(), None).await.unwrap(),
        );
        assert_eq!(a.marks, b.marks);
        assert_ne!(a.id, b.id, "each run is its own territory");
    }

    /// A one-page Bundle beside getting-started: its schema and manifest, a
    /// page and a paragraph of its own.
    fn one_page_bundle() -> transload::Bundle {
        const PAGE: &str = "library/builtin.page";
        let seed = store::collections::getting_started::bundle();
        let mut bundle = transload::Bundle::new();
        for path in ["manifest.json", &format!("{PAGE}/_schema.json")] {
            bundle.insert(path, seed.get(path).expect(path));
        }
        let (page, para) = (Uuid::new_v4(), Uuid::new_v4());
        let stamp = r#""created":"2026-11-23T00:00:00.000Z","updated":"2026-11-23T00:00:00.000Z""#;
        bundle.insert(
            "marks.jsonl",
            format!("{{\"id\":\"{page}\",{stamp}}}\n{{\"id\":\"{para}\",{stamp}}}\n"),
        );
        bundle.insert(
            format!("{PAGE}/{page}.md"),
            format!(
                "---\ntitle: |-\n  Listing\nchart: builtin.page\ncreated: 2026-11-23T00:00:00.000Z\nupdated: 2026-11-23T00:00:00.000Z\n---\n\n<!-- wp:mark {para} -->\nA page of its own.\n"
            ),
        );
        bundle
    }

    #[tokio::test]
    async fn a_further_bundle_is_ingested_through_the_same_path() {
        let source = tempfile::tempdir().unwrap();
        one_page_bundle().write_dir(source.path()).unwrap();

        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let base = write(a.path(), None).await.unwrap();
        let with = write(b.path(), Some(source.path())).await.unwrap();
        assert_eq!(with.marks, base.marks + 2);
    }

    #[tokio::test]
    async fn refuses_a_non_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("stray"), "").unwrap();
        assert!(write(tmp.path(), None).await.is_err());
    }
}
