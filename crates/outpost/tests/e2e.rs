//! Full-stack proof: two real app databases, two coordinators, the real
//! `HttpRelayClient`, and a live outpost — the complete sync path over
//! actual HTTP.

use std::sync::Arc;

use cartograph::Maneuver;
use cartograph::sync::{HttpRelayClient, SyncCoordinator, sync_once, touched_marks};
use domain::mark::{Content, MarkData, MarkID, RichText};
use domain::sync::{AccountID, DeviceID, RealmID};
use outpost::auth::Validator;
use outpost::storage::Storage;
use outpost::{AppState, router};
use store::DatabaseConnection;
use store::query::{device, mark as mark_q};
use store::test_utils::temp_db;

const SECRET: &str = "test-outpost-secret";

fn paragraph(text: &str) -> MarkData {
    MarkData::Content(Content::paragraph(RichText::from(text.to_string())))
}

async fn spawn_outpost() -> String {
    let dir = tempfile_dir();
    let storage = Storage::open(dir.join("outpost.db").to_str().unwrap())
        .await
        .expect("open outpost storage");
    let state = AppState {
        storage: Arc::new(storage),
        validator: Arc::new(Validator::SharedSecret(SECRET.into())),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    format!("http://{addr}")
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("outpost-e2e-{}", uuid_ish()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn uuid_ish() -> String {
    format!("{:x}", std::process::id() as u128 ^ nanos())
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

async fn joined_device(
    account: AccountID,
    realm: RealmID,
    name: &str,
) -> (DatabaseConnection, SyncCoordinator) {
    let db = temp_db().await;
    device::insert(&db, DeviceID::new(), account, realm, name)
        .await
        .unwrap();
    let coordinator = SyncCoordinator::load(&db).await.unwrap();
    (db, coordinator)
}

async fn create_mark(
    db: &DatabaseConnection,
    coordinator: &mut SyncCoordinator,
    text: &str,
) -> MarkID {
    let id = MarkID::new();
    mark_q::insert(db, id, &paragraph(text), chrono::Local::now().naive_utc())
        .await
        .unwrap();
    let ops = vec![Maneuver::CreateMark {
        mark_id: id,
        data: paragraph(text),
    }];
    let marks = touched_marks(&ops);
    coordinator.log_local(db, ops, marks).await.unwrap();
    id
}

#[tokio::test]
async fn two_devices_sync_over_http() {
    let base = spawn_outpost().await;
    let relay = HttpRelayClient::new(&base, SECRET);

    let (account, realm) = (AccountID::new(), RealmID::new());
    let (db_a, mut a) = joined_device(account, realm, "a").await;
    let (db_b, mut b) = joined_device(account, realm, "b").await;

    let id = create_mark(&db_a, &mut a, "hello over http").await;
    let up = sync_once(&db_a, &mut a, &relay, &base).await.unwrap();
    assert_eq!(up.uploaded, 1);

    let down = sync_once(&db_b, &mut b, &relay, &base).await.unwrap();
    assert_eq!((down.fetched, down.applied), (1, 1));
    let on_b = mark_q::get(&db_b, id).await.unwrap().unwrap();
    assert_eq!(on_b.data, paragraph("hello over http"));

    // Idempotent re-sync: quiescent everywhere.
    let quiet = sync_once(&db_b, &mut b, &relay, &base).await.unwrap();
    assert_eq!(quiet.fetched, 0);

    // HLC survives the JSON round-trip bit-for-bit (string wire format).
    let clock = mark_q::get_row_clock(&db_b, id).await.unwrap().unwrap();
    let local = mark_q::get_row_clock(&db_a, id).await.unwrap().unwrap();
    assert_eq!(clock, local, "clocks must round-trip exactly");
}

#[tokio::test]
async fn wrong_secret_is_rejected() {
    let base = spawn_outpost().await;
    let relay = HttpRelayClient::new(&base, "not-the-secret");

    let (account, realm) = (AccountID::new(), RealmID::new());
    let (db_a, mut a) = joined_device(account, realm, "a").await;
    create_mark(&db_a, &mut a, "sealed").await;

    let err = sync_once(&db_a, &mut a, &relay, &base).await.unwrap_err();
    assert!(
        err.to_string().contains("401"),
        "expected 401 rejection, got: {err}"
    );
}
