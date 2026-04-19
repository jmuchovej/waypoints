//! Sync persistence layer (#130): device + sync_log tables and row clocks.

mod fixtures;

use domain::sync::{DeviceID, HLC};
use domain::terrain::{TerrainID, TerrainKind};
use fixtures::*;
use rstest::rstest;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use store::StoreError;
use store::entities::{ActiveDevice, ActiveSyncLog, Devices, SyncLog, SyncLogColumn};
use store::query::{mark, terrain};
use uuid::Uuid;

// ── Row clocks ──────────────────────────────────────────────────────────

/// Rows created outside the sync layer have `updated_hlc = 0`, which
/// `get_row_clock` reports as `None` ("never touched by sync").
#[rstest]
#[tokio::test]
async fn row_clocks_default_to_none(#[future] db: DatabaseConnection) {
    let db = db.await;
    let id = new_id();
    let parent = new_id();
    mark::insert(&db, id, &paragraph("block"), now())
        .await
        .unwrap();
    mark::insert(&db, parent, &landmark("page", "Parent"), now())
        .await
        .unwrap();
    let edge_id = TerrainID::new();
    terrain::insert_edge(&db, edge_id, parent, id, &TerrainKind::Child, None, None)
        .await
        .unwrap();

    assert_eq!(mark::get_row_clock(&db, id).await.unwrap(), None);
    assert_eq!(terrain::get_row_clock(&db, edge_id).await.unwrap(), None);

    // Missing rows also read as "never touched".
    assert_eq!(mark::get_row_clock(&db, new_id()).await.unwrap(), None);
    assert_eq!(
        terrain::get_row_clock(&db, TerrainID::new()).await.unwrap(),
        None
    );
}

#[rstest]
#[tokio::test]
async fn row_clocks_round_trip(#[future] db: DatabaseConnection) {
    let db = db.await;
    let id = new_id();
    let parent = new_id();
    mark::insert(&db, id, &paragraph("block"), now())
        .await
        .unwrap();
    mark::insert(&db, parent, &landmark("page", "Parent"), now())
        .await
        .unwrap();
    let edge_id = TerrainID::new();
    terrain::insert_edge(&db, edge_id, parent, id, &TerrainKind::Child, None, None)
        .await
        .unwrap();

    let device = DeviceID::new();
    mark::set_row_clock(&db, id, HLC(42), device).await.unwrap();
    terrain::set_row_clock(&db, edge_id, HLC(43), device)
        .await
        .unwrap();

    assert_eq!(
        mark::get_row_clock(&db, id).await.unwrap(),
        Some((HLC(42), device))
    );
    assert_eq!(
        terrain::get_row_clock(&db, edge_id).await.unwrap(),
        Some((HLC(43), device))
    );
}

#[rstest]
#[tokio::test]
async fn set_row_clock_errors_on_missing_rows(#[future] db: DatabaseConnection) {
    let db = db.await;
    let device = DeviceID::new();

    let missing_mark = new_id();
    assert!(matches!(
        mark::set_row_clock(&db, missing_mark, HLC(1), device).await,
        Err(StoreError::MarkNotFound(id)) if id == missing_mark
    ));

    let missing_edge = TerrainID::new();
    assert!(matches!(
        terrain::set_row_clock(&db, missing_edge, HLC(1), device).await,
        Err(StoreError::EdgeNotFound(id)) if id == missing_edge
    ));
}

/// Soft-deleted marks keep a writable clock — deletion is itself a
/// synced write, and a later remote op may still need to dominate it.
#[rstest]
#[tokio::test]
async fn row_clock_works_on_soft_deleted_marks(#[future] db: DatabaseConnection) {
    let db = db.await;
    let id = new_id();
    mark::insert(&db, id, &paragraph("block"), now())
        .await
        .unwrap();
    mark::soft_delete(&db, id).await.unwrap();

    let device = DeviceID::new();
    mark::set_row_clock(&db, id, HLC(7), device).await.unwrap();
    assert_eq!(
        mark::get_row_clock(&db, id).await.unwrap(),
        Some((HLC(7), device))
    );
}

// ── device table ────────────────────────────────────────────────────────

#[rstest]
#[tokio::test]
async fn device_row_round_trips(#[future] db: DatabaseConnection) {
    let db = db.await;
    let device_id = Uuid::new_v4();

    ActiveDevice {
        device_id: Set(device_id),
        account_id: Set(Uuid::new_v4()),
        realm_id: Set(Uuid::new_v4()),
        friendly_name: Set("test-host".into()),
        last_hlc: Set(0),
        alias_of: Set(None),
        created_at: Set(now()),
        retired_at: Set(None),
    }
    .insert(&db)
    .await
    .unwrap();

    let row = Devices::find_by_id(device_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.friendly_name, "test-host");
    assert_eq!(row.last_hlc, 0);
    assert_eq!(row.alias_of, None);
    assert_eq!(row.retired_at, None);
}

// ── sync_log table ──────────────────────────────────────────────────────

/// The relay-cursor query shape: all groups in a realm with hlc > X,
/// ordered by hlc.
#[rstest]
#[tokio::test]
async fn sync_log_serves_the_since_cursor_query(#[future] db: DatabaseConnection) {
    let db = db.await;
    let realm = Uuid::new_v4();
    let device = Uuid::new_v4();
    let account = Uuid::new_v4();

    for hlc in [10i64, 20, 30] {
        ActiveSyncLog {
            group_id: Set(Uuid::new_v4()),
            device_id: Set(device),
            account_id: Set(account),
            realm_id: Set(realm),
            hlc: Set(hlc),
            ops: Set(Some(serde_json::json!([]))),
            touched_marks: Set(Some(serde_json::json!([]))),
            payload: Set(Some("{}".into())),
            applied_at: Set(Some(now())),
            origin: Set(0),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
    }

    let since = SyncLog::find()
        .filter(SyncLogColumn::RealmId.eq(realm))
        .filter(SyncLogColumn::Hlc.gt(10))
        .order_by_asc(SyncLogColumn::Hlc)
        .all(&db)
        .await
        .unwrap();

    assert_eq!(since.len(), 2);
    assert_eq!(since[0].hlc, 20);
    assert_eq!(since[1].hlc, 30);
}

/// group_id is the idempotency key: inserting the same group twice fails
/// at the schema level.
#[rstest]
#[tokio::test]
async fn sync_log_rejects_duplicate_group_ids(#[future] db: DatabaseConnection) {
    let db = db.await;
    let group_id = Uuid::new_v4();

    let row = || ActiveSyncLog {
        group_id: Set(group_id),
        device_id: Set(Uuid::new_v4()),
        account_id: Set(Uuid::new_v4()),
        realm_id: Set(Uuid::new_v4()),
        hlc: Set(1),
        ops: Set(Some(serde_json::json!([]))),
        touched_marks: Set(Some(serde_json::json!([]))),
        payload: Set(Some("{}".into())),
        applied_at: Set(Some(now())),
        origin: Set(1),
        ..Default::default()
    };

    row().insert(&db).await.unwrap();
    assert!(row().insert(&db).await.is_err());
}

// ── Relay sync state (#132) ─────────────────────────────────────────────

#[rstest]
#[tokio::test]
async fn relay_cursors_are_per_source(#[future] db: DatabaseConnection) {
    use domain::sync::RealmID;
    use store::query::relay_cursor;

    let db = db.await;
    let realm = RealmID::new();

    assert_eq!(relay_cursor::get(&db, "official", realm).await.unwrap(), 0);

    relay_cursor::set(&db, "official", realm, 42).await.unwrap();
    relay_cursor::set(&db, "homelab", realm, 7).await.unwrap();

    assert_eq!(relay_cursor::get(&db, "official", realm).await.unwrap(), 42);
    assert_eq!(relay_cursor::get(&db, "homelab", realm).await.unwrap(), 7);

    // upsert overwrites
    relay_cursor::set(&db, "official", realm, 43).await.unwrap();
    assert_eq!(relay_cursor::get(&db, "official", realm).await.unwrap(), 43);
}

#[rstest]
#[tokio::test]
async fn outbox_tracks_pending_and_acked_uploads(#[future] db: DatabaseConnection) {
    use domain::sync::{AccountID, DeviceID, HLC, OpID, RealmID};
    use store::query::sync_log::{self, SyncLogEntry, SyncOrigin};

    let db = db.await;
    let realm = RealmID::new();
    let device = DeviceID::new();
    let account = AccountID::new();

    let entry = |hlc: i64, origin: SyncOrigin| SyncLogEntry {
        group_id: OpID::new(),
        device_id: device,
        account_id: account,
        realm_id: realm,
        hlc: HLC(hlc),
        ops: serde_json::json!([]),
        touched_marks: serde_json::json!([]),
        payload: "{}".into(),
        origin,
    };

    let local_a = entry(10, SyncOrigin::Local);
    let local_b = entry(20, SyncOrigin::Local);
    let remote = entry(15, SyncOrigin::Remote);
    let (a_id, b_id) = (local_a.group_id, local_b.group_id);
    for e in [&local_a, &local_b, &remote] {
        sync_log::insert(&db, e.clone()).await.unwrap();
    }

    // Only local-origin, unacked rows, oldest HLC first.
    let pending = sync_log::pending_upload(&db, realm, 10).await.unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].group_id, a_id.inner());
    assert_eq!(pending[1].group_id, b_id.inner());

    sync_log::mark_uploaded(&db, &[a_id]).await.unwrap();
    let pending = sync_log::pending_upload(&db, realm, 10).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].group_id, b_id.inner());
}
