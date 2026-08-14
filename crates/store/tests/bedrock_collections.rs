//! Round-trip tests for the bedrock-backed collections.
//!
//! Each test opens a `Store` on a tempdir path, writes domain types via the
//! bedrock `Database`, reads them back, and asserts the round-trip. Tests run
//! against a live turso engine — no string-only assertions.

use chrono::{NaiveDate, NaiveDateTime};
use domain::{Content, Mark, MarkData, MarkID, Terrain, TerrainID, TerrainKind};
use store::Store;
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
    let mark = Mark {
        id,
        data: data.clone(),
        extra: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    db.insert(mark).await.expect("insert mark");

    let fetched = db
        .get::<Mark>(&id)
        .await
        .expect("get mark")
        .expect("mark must exist");

    // The before_save hook stamps updated_at, so we only compare fields that
    // shouldn't change.
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.data, data);
    assert_eq!(fetched.extra, None);
    assert_eq!(fetched.created_at, now);
    assert!(fetched.deleted_at.is_none());
}

#[tokio::test]
async fn mark_with_extra_roundtrips() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    let id = MarkID::new();
    let data = MarkData::Content(Content::default());
    let extra = serde_json::json!({ "custom": true, "count": 42 });
    let now = at(100);
    let mark = Mark {
        id,
        data: data.clone(),
        extra: Some(extra.clone()),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };

    db.insert(mark).await.expect("insert");

    let fetched = db.get::<Mark>(&id).await.expect("get").expect("exists");
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.data, data);
    assert_eq!(fetched.extra, Some(extra));
}

// ── Chart round-trip ─────────────────────────────────────────────────────────

#[tokio::test]
async fn builtin_charts_are_seeded_on_open() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    // 9 builtin charts: page, objective, note, person, campaign, meeting,
    // bookmark, journal, book.
    let count = db.count::<domain::Chart>().await.expect("count");
    assert_eq!(count, 9, "expected exactly 9 builtin charts, got {count}");
}

#[tokio::test]
async fn chart_domain_roundtrip() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    // Fetch the page chart (always seeded).
    let page_id = store::builtin::page::type_id();
    let chart = db
        .get::<domain::Chart>(&page_id)
        .await
        .expect("get")
        .expect("page chart must be seeded");

    assert_eq!(chart.id, page_id);
    assert_eq!(chart.name, "Page");
}

#[tokio::test]
async fn seed_is_idempotent() {
    let (store, _dir) = open_store().await;
    let db = store.db();

    // Run seed again — should not error or duplicate rows.
    store::collections::seed::run(db).await.expect("re-seed");

    let count = db.count::<domain::Chart>().await.expect("count");
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
    db.insert(Mark {
        id: src,
        data: MarkData::Content(Content::default()),
        extra: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    })
    .await
    .expect("insert src mark");
    db.insert(Mark {
        id: dst,
        data: MarkData::Content(Content::default()),
        extra: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    })
    .await
    .expect("insert dst mark");

    let terrain = Terrain {
        id: TerrainID::new(),
        src_id: src,
        dst_id: dst,
        kind: TerrainKind::Child,
        order: None,
        extra: None,
        deleted_at: None,
    };
    let terrain_id = terrain.id;

    db.insert(terrain).await.expect("insert terrain");

    let fetched = db
        .get::<Terrain>(&terrain_id)
        .await
        .expect("get terrain")
        .expect("terrain must exist");

    assert_eq!(fetched.id, terrain_id);
    assert_eq!(fetched.src_id, src);
    assert_eq!(fetched.dst_id, dst);
    assert_eq!(fetched.kind, TerrainKind::Child);
    assert!(fetched.deleted_at.is_none());
    assert!(fetched.order.is_none());
    assert!(fetched.extra.is_none());
}

#[tokio::test]
async fn terrain_reading_kind_roundtrips() {
    use domain::FeatureID;
    use std::str::FromStr;

    let (store, _dir) = open_store().await;
    let db = store.db();

    let src = MarkID::new();
    let dst = MarkID::new();
    let now = at(0);
    db.insert(Mark {
        id: src,
        data: MarkData::Content(Content::default()),
        extra: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    })
    .await
    .expect("insert src");
    db.insert(Mark {
        id: dst,
        data: MarkData::Content(Content::default()),
        extra: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    })
    .await
    .expect("insert dst");

    let chart_id = store::builtin::page::type_id();
    let feature_id = FeatureID::from_str("a1b2").unwrap();
    let kind = TerrainKind::Reading(chart_id.clone(), feature_id.clone());

    let terrain_id = TerrainID::new();
    db.insert(Terrain {
        id: terrain_id,
        src_id: src,
        dst_id: dst,
        kind: kind.clone(),
        order: None,
        extra: None,
        deleted_at: None,
    })
    .await
    .expect("insert terrain");

    let fetched = db
        .get::<Terrain>(&terrain_id)
        .await
        .expect("get")
        .expect("exists");

    assert_eq!(fetched.kind, kind);
}
