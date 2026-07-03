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
            ops: Set(serde_json::json!([])),
            touched_marks: Set(serde_json::json!([])),
            applied_at: Set(now()),
            origin: Set(0),
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
        ops: Set(serde_json::json!([])),
        touched_marks: Set(serde_json::json!([])),
        applied_at: Set(now()),
        origin: Set(1),
    };

    row().insert(&db).await.unwrap();
    assert!(row().insert(&db).await.is_err());
}
