//! Full-stack proof: two real app databases, two coordinators, the real
//! `HttpPeer`, and a live outpost — the complete sync path over
//! actual HTTP. Plus the gossip pump: two live outposts converging
//! through pull-only anti-entropy (PROTOCOL.md §1 "Gossip").

use std::sync::Arc;

use cartograph::Maneuver;
use cartograph::sync::{HttpPeer, SyncCoordinator, sync_once, touched_marks};
use domain::mark::{Content, MarkData, MarkID, RichText};
use domain::sync::{AccountID, DeviceID, RealmID};
use outpost::auth::Validator;
use outpost::sync::peers::pump::{pump_all, pump_once};
use outpost::sync::peers::store::EnvelopeStore;
use outpost::sync::peers::turso::Storage;
use outpost::{AppState, router};
use store::DatabaseConnection;
use store::query::{device, mark as mark_q};
use store::test_utils::temp_db;

const SECRET: &str = "test-outpost-secret";

fn paragraph(text: &str) -> MarkData {
    MarkData::Content(Content::paragraph(RichText::from(text.to_string())))
}

async fn spawn_outpost() -> (String, Arc<Storage>) {
    let dir = tempfile_dir();
    let storage = Arc::new(
        Storage::open(dir.join("outpost.db").to_str().unwrap())
            .await
            .expect("open outpost storage"),
    );
    let state = AppState {
        storage: storage.clone(),
        validator: Arc::new(Validator::SharedSecret(SECRET.into())),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    (format!("http://{addr}"), storage)
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("outpost-e2e-{}", uuid_ish()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn uuid_ish() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{:x}-{n}", std::process::id() as u128 ^ nanos())
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
    let (base, _storage) = spawn_outpost().await;
    let relay = HttpPeer::new(&base, SECRET);

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
    let (base, _storage) = spawn_outpost().await;
    let relay = HttpPeer::new(&base, "not-the-secret");

    let (account, realm) = (AccountID::new(), RealmID::new());
    let (db_a, mut a) = joined_device(account, realm, "a").await;
    create_mark(&db_a, &mut a, "sealed").await;

    let err = sync_once(&db_a, &mut a, &relay, &base).await.unwrap_err();
    assert!(
        err.to_string().contains("401"),
        "expected 401 rejection, got: {err}"
    );
}

// ── Gossip pump ─────────────────────────────────────────────────────────

#[tokio::test]
async fn pump_converges_two_outposts() {
    let (base_a, _storage_a) = spawn_outpost().await;
    let (base_b, storage_b) = spawn_outpost().await;

    let (account, realm) = (AccountID::new(), RealmID::new());
    let (db_1, mut dev_1) = joined_device(account, realm, "one").await;
    let (db_2, mut dev_2) = joined_device(account, realm, "two").await;

    // Device 1 only ever talks to outpost A.
    let id = create_mark(&db_1, &mut dev_1, "via gossip").await;
    let relay_a = HttpPeer::new(&base_a, SECRET);
    sync_once(&db_1, &mut dev_1, &relay_a, &base_a)
        .await
        .unwrap();

    // B pulls from A.
    let peer_a = HttpPeer::new(&base_a, SECRET);
    let realm_str = realm.to_string();
    let pumped = pump_once(storage_b.as_ref(), &peer_a, &realm_str)
        .await
        .unwrap();
    assert_eq!((pumped.fetched, pumped.stored, pumped.dropped), (1, 1, 0));

    // Device 2 only ever talks to outpost B — and still converges.
    let relay_b = HttpPeer::new(&base_b, SECRET);
    let down = sync_once(&db_2, &mut dev_2, &relay_b, &base_b)
        .await
        .unwrap();
    assert_eq!((down.fetched, down.applied), (1, 1));
    let on_2 = mark_q::get(&db_2, id).await.unwrap().unwrap();
    assert_eq!(on_2.data, paragraph("via gossip"));

    // Re-pump is a no-op: the cursor is past everything A holds.
    let again = pump_once(storage_b.as_ref(), &peer_a, &realm_str)
        .await
        .unwrap();
    assert_eq!(again.fetched, 0);
}

#[tokio::test]
async fn bidirectional_pump_reaches_fixpoint() {
    let (base_a, storage_a) = spawn_outpost().await;
    let (base_b, storage_b) = spawn_outpost().await;

    let (account, realm) = (AccountID::new(), RealmID::new());
    let (db_1, mut dev_1) = joined_device(account, realm, "one").await;
    let (db_2, mut dev_2) = joined_device(account, realm, "two").await;

    // Each device edits against its own outpost.
    let id_1 = create_mark(&db_1, &mut dev_1, "from A's side").await;
    let id_2 = create_mark(&db_2, &mut dev_2, "from B's side").await;
    let relay_a = HttpPeer::new(&base_a, SECRET);
    let relay_b = HttpPeer::new(&base_b, SECRET);
    sync_once(&db_1, &mut dev_1, &relay_a, &base_a)
        .await
        .unwrap();
    sync_once(&db_2, &mut dev_2, &relay_b, &base_b)
        .await
        .unwrap();

    let peer_a = HttpPeer::new(&base_a, SECRET);
    let peer_b = HttpPeer::new(&base_b, SECRET);
    let realm_str = realm.to_string();

    // A↔B gossip cycle: idempotency terminates propagation. Two laps
    // reach the fixpoint; a third fetches nothing anywhere.
    for _ in 0..2 {
        pump_once(storage_b.as_ref(), &peer_a, &realm_str)
            .await
            .unwrap();
        pump_once(storage_a.as_ref(), &peer_b, &realm_str)
            .await
            .unwrap();
    }
    let quiet_b = pump_once(storage_b.as_ref(), &peer_a, &realm_str)
        .await
        .unwrap();
    let quiet_a = pump_once(storage_a.as_ref(), &peer_b, &realm_str)
        .await
        .unwrap();
    assert_eq!(quiet_b.fetched, 0, "B has everything A holds");
    assert_eq!(quiet_a.fetched, 0, "A has everything B holds");

    // No envelope inflation: both hold exactly the two envelopes, once.
    let on_a = storage_a.page(&realm_str, 0, 10).await.unwrap();
    let on_b = storage_b.page(&realm_str, 0, 10).await.unwrap();
    assert_eq!((on_a.len(), on_b.len()), (2, 2));

    // Both devices converge through their own outpost only.
    sync_once(&db_1, &mut dev_1, &relay_a, &base_a)
        .await
        .unwrap();
    sync_once(&db_2, &mut dev_2, &relay_b, &base_b)
        .await
        .unwrap();
    assert_eq!(
        mark_q::get(&db_1, id_2).await.unwrap().unwrap().data,
        paragraph("from B's side")
    );
    assert_eq!(
        mark_q::get(&db_2, id_1).await.unwrap().unwrap().data,
        paragraph("from A's side")
    );
}

#[tokio::test]
async fn pump_all_bootstraps_fresh_mirror_via_extra_realms() {
    let (base_a, _storage_a) = spawn_outpost().await;
    let (_base_b, storage_b) = spawn_outpost().await;

    let (account, realm) = (AccountID::new(), RealmID::new());
    let (db_1, mut dev_1) = joined_device(account, realm, "one").await;
    create_mark(&db_1, &mut dev_1, "mirrored").await;
    let relay_a = HttpPeer::new(&base_a, SECRET);
    sync_once(&db_1, &mut dev_1, &relay_a, &base_a)
        .await
        .unwrap();

    // B is brand-new: it knows no realms locally, so only the configured
    // extras give it something to pull (the bootstrap case).
    let peers = vec![HttpPeer::new(&base_a, SECRET)];

    let nothing = pump_all(storage_b.as_ref(), &peers, &[]).await;
    assert_eq!(nothing.fetched, 0, "no local realms, no extras: no pull");

    let realm_str = realm.to_string();
    let bootstrapped = pump_all(storage_b.as_ref(), &peers, std::slice::from_ref(&realm_str)).await;
    assert_eq!((bootstrapped.fetched, bootstrapped.stored), (1, 1));

    // Once mirrored, the realm is locally known: extras no longer needed.
    create_mark(&db_1, &mut dev_1, "second wave").await;
    sync_once(&db_1, &mut dev_1, &relay_a, &base_a)
        .await
        .unwrap();
    let steady = pump_all(storage_b.as_ref(), &peers, &[]).await;
    assert_eq!((steady.fetched, steady.stored), (1, 1));

    let held = storage_b.page(&realm_str, 0, 10).await.unwrap();
    assert_eq!(held.len(), 2);
}
