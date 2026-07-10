//! Round-trip tests for the bedrock-backed collections.
//!
//! Each test opens a `Store` on a tempdir path, writes rows via the bedrock
//! `Database`, reads them back, and asserts the domain round-trip. Tests run
//! against a live turso engine — no string-only assertions.

use chrono::{NaiveDate, NaiveDateTime};
use domain::{
    Terrain, TerrainKind,
    mark::{Content, Mark, MarkData, MarkID},
    terrain::TerrainID,
};
use store::{
    Store,
    collections::{ChartRow, MarkRow, TerrainRow},
};
use tempfile::tempdir;

fn at(milli: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 7, 14)
        .unwrap()
        .and_hms_milli_opt(10, 0, 0, milli)
        .unwrap()
}

/// Open a fresh `Store` in a temporary directory.
async fn open_store() -> (Store, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("test.db"))
        .await
        .expect("Store::open");
    (store, dir)
}

// ── Mark round-trip ──────────────────────────────────────────────────────────

#[tokio::test]
async fn mark_insert_get_roundtrip() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    let id = MarkID::new();
    let data = MarkData::Content(Content::default());
    let now = at(0);
    let row = MarkRow::from_parts(id, data.clone(), now);

    db.insert(row.clone()).await.expect("insert mark");

    let fetched = db
        .get::<MarkRow>(&id)
        .await
        .expect("get mark")
        .expect("mark must exist");

    // The before_save hook stamps updated_at, so we only compare fields that
    // shouldn't change.
    assert_eq!(fetched.id, row.id);
    assert_eq!(fetched.data, row.data);
    assert_eq!(fetched.extra, row.extra);
    assert_eq!(fetched.created_at, row.created_at);

    // Domain round-trip.
    let mark = Mark::try_from(fetched).expect("Mark::try_from");
    assert_eq!(mark.id, id);
    assert_eq!(mark.data, data);
    assert!(mark.extra.is_none());
}

#[tokio::test]
async fn mark_with_extra_roundtrips() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    let id = MarkID::new();
    let data = MarkData::Content(Content::default());
    let extra = serde_json::json!({ "custom": true, "count": 42 });
    let now = at(100);
    let mut row = MarkRow::from_parts(id, data.clone(), now);
    row.extra = Some(extra.clone());

    db.insert(row).await.expect("insert");

    let fetched = db.get::<MarkRow>(&id).await.expect("get").expect("exists");
    assert_eq!(fetched.extra, Some(extra.clone()));

    let mark = Mark::try_from(fetched).expect("try_from");
    assert_eq!(mark.extra, Some(extra));
}

// ── Chart round-trip ─────────────────────────────────────────────────────────

#[tokio::test]
async fn builtin_charts_are_seeded_on_open() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    // 9 builtin charts: page, objective, note, person, campaign, meeting,
    // bookmark, journal, book.
    let count = db.count::<ChartRow>().await.expect("count");
    assert_eq!(count, 9, "expected exactly 9 builtin charts, got {count}");
}

#[tokio::test]
async fn chart_domain_roundtrip() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    // Fetch the page chart (always seeded).
    let page_id = store::builtin::page::type_id();
    let row = db
        .get::<ChartRow>(&page_id)
        .await
        .expect("get")
        .expect("page chart must be seeded");

    let chart = domain::Chart::from(row);
    assert_eq!(chart.id, page_id);
    assert_eq!(chart.name, "Page");
}

#[tokio::test]
async fn seed_is_idempotent() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    // Run seed again — should not error or duplicate rows.
    store::collections::seed::run(db).await.expect("re-seed");

    let count = db.count::<ChartRow>().await.expect("count");
    assert_eq!(count, 9);
}

// ── Terrain round-trip ───────────────────────────────────────────────────────

#[tokio::test]
async fn terrain_insert_get_roundtrip() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    let src = MarkID::new();
    let dst = MarkID::new();

    // We need the mark rows to exist first (foreign keys aren't enforced in bedrock
    // STRICT tables without an explicit FK constraint, but let's be explicit).
    let now = at(0);
    db.insert(MarkRow::from_parts(
        src,
        MarkData::Content(Content::default()),
        now,
    ))
    .await
    .expect("insert src mark");
    db.insert(MarkRow::from_parts(
        dst,
        MarkData::Content(Content::default()),
        now,
    ))
    .await
    .expect("insert dst mark");

    let terrain = Terrain::builder()
        .with_src_id(src)
        .with_dst_id(dst)
        .with_kind(TerrainKind::Child)
        .build();

    let row = TerrainRow::from_terrain(&terrain);
    let terrain_id = row.id;

    db.insert(row).await.expect("insert terrain");

    let fetched = db
        .get::<TerrainRow>(&terrain_id)
        .await
        .expect("get terrain")
        .expect("terrain must exist");

    assert_eq!(fetched.id, terrain_id);
    assert_eq!(fetched.src_id, src);
    assert_eq!(fetched.dst_id, dst);
    assert_eq!(fetched.kind, "child");
    assert!(fetched.deleted_at.is_none());

    // Domain round-trip.
    let back: Terrain = fetched.into();
    assert_eq!(back.id, terrain_id);
    assert_eq!(back.src_id, src);
    assert_eq!(back.dst_id, dst);
    assert_eq!(back.kind, TerrainKind::Child);
    assert!(back.order.is_none());
    assert!(back.extra.is_none());
}

#[tokio::test]
async fn terrain_reading_kind_roundtrips() {
    use domain::chart::FeatureID;
    use std::str::FromStr;

    let (store, _dir) = open_store().await;
    let db = store.db();

    let src = MarkID::new();
    let dst = MarkID::new();
    let now = at(0);
    db.insert(MarkRow::from_parts(
        src,
        MarkData::Content(Content::default()),
        now,
    ))
    .await
    .expect("insert src");
    db.insert(MarkRow::from_parts(
        dst,
        MarkData::Content(Content::default()),
        now,
    ))
    .await
    .expect("insert dst");

    let chart_id = store::builtin::page::type_id();
    let feature_id = FeatureID::from_str("a1b2").unwrap();
    let kind = TerrainKind::Reading(chart_id.clone(), feature_id.clone());

    let terrain = Terrain::builder()
        .with_src_id(src)
        .with_dst_id(dst)
        .with_kind(kind.clone())
        .build();

    let terrain_id = terrain.id;
    let row = TerrainRow::from_terrain(&terrain);
    // The kind string should encode the full builtin prefix.
    assert!(row.kind.starts_with("reading::builtin.page::"));

    db.insert(row).await.expect("insert terrain");

    let fetched = db
        .get::<TerrainRow>(&terrain_id)
        .await
        .expect("get")
        .expect("exists");

    let back: Terrain = fetched.into();
    assert_eq!(back.kind, kind);
}
