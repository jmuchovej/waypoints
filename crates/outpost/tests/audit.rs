//! Hand-run audits against a *deployed* Station — a pod on kind, a
//! container — where a Vessel has to come from somewhere and the app is not
//! installed. Ignored by default; `OUTPOST_AUDIT_ADDR` names the Station as
//! either its JSON address (the "station running" log line's `address`
//! field) or the path of a `station.addr` file:
//!
//!   OUTPOST_AUDIT_ADDR=/path/to/station.addr \
//!     cargo test -p outpost --test audit -- --ignored --nocapture <name>
//!
//! The Vessel binds routable so it can dial a non-loopback address too.

#![allow(
    clippy::disallowed_macros,
    reason = "an operator reads this at a terminal; there is no runtime to swallow stdout"
)]

use outpost::{
    fabric::EndpointAddr,
    protocol::{EpochID, KeyRoot, NoticeID},
    replica::Replica,
    vessel::{SubscriptionEvent, Vessel},
};
use rand::SeedableRng;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use tokio::time::{sleep, timeout};

fn station() -> EndpointAddr {
    let value = std::env::var("OUTPOST_AUDIT_ADDR")
        .expect("OUTPOST_AUDIT_ADDR: a station.addr path or its JSON");
    let json = if Path::new(&value).is_file() {
        std::fs::read_to_string(&value).expect("read the address file")
    } else {
        value
    };
    serde_json::from_str(json.trim()).expect("a JSON EndpointAddr")
}

/// `OUTPOST_AUDIT_WAIT` seconds (default 300) — how long to wait for the
/// operator's side of an audit (deleting a pod, scaling, …).
fn operator_wait() -> Duration {
    Duration::from_secs(
        std::env::var("OUTPOST_AUDIT_WAIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300),
    )
}

/// A Territory with one authored Notice, and a routable second device of
/// the owner holding both — the device that does the talking.
async fn territory_and_device() -> (Vessel, NoticeID, String) {
    territory_and_device_from(0xA7).await
}

/// [`territory_and_device`] for an owner derived from `seed` — distinct
/// seeds give distinct identities and so distinct Territories.
async fn territory_and_device_from(seed: u32) -> (Vessel, NoticeID, String) {
    let mut bytes = [0xA7u8; 64];
    bytes[..4].copy_from_slice(&seed.to_le_bytes());
    let mut rng = rand::rngs::StdRng::from_seed(bytes[..32].try_into().unwrap());
    let root = KeyRoot::from_seed(bytes);
    let creator = Vessel::create_territory(&root.identity(), &root.device(0), ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = creator.territory_id().await.unwrap();
    let notice = creator
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"audit",
        )
        .await
        .unwrap();
    let notice_id = NoticeID::of(&notice).unwrap();
    let acl = creator.replica().hold().acl_records().await.unwrap();

    let device = Vessel::from_identity_routable(
        &root.identity(),
        &root.device(1),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    device.replica().ingest(vec![notice]).await.unwrap();
    (device, notice_id, hex::encode(territory.as_bytes()))
}

/// Does the Station hold `notice_id` for the Territory `acl` describes? A
/// fresh device that only knows the ACL log syncs and looks.
async fn station_holds(addr: &EndpointAddr, device: &Vessel, notice_id: NoticeID) -> bool {
    let root = KeyRoot::from_seed([0xA7; 64]);
    let acl = device.replica().hold().acl_records().await.unwrap();
    let mut fresh = Vessel::from_identity_routable(
        &root.identity(),
        &root.device(2),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    fresh.sync_with(addr.clone()).await.unwrap();
    fresh
        .replica()
        .hold()
        .get_notice(notice_id)
        .await
        .unwrap()
        .is_some()
}

/// Register a Territory, push a Notice, and confirm the Station serves it
/// to a device that never held it. Prints the Territory's hold file name.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "drives the deployed Station named by OUTPOST_AUDIT_ADDR"]
async fn register_and_sync() {
    let addr = station();
    let (mut device, notice_id, territory_hex) = territory_and_device().await;
    let territory = device.territory_id().await.unwrap();

    device.register(addr.clone(), territory).await.unwrap();
    device.sync_with(addr.clone()).await.unwrap();
    assert!(station_holds(&addr, &device, notice_id).await);
    println!("registered and synced; hold file: territories/{territory_hex}.db");
}

/// Home `OUTPOST_AUDIT_TERRITORIES` (default 200) distinct Territories on
/// the Station, so that its next start has a scan worth watching: the
/// window in which `/readyz` answers 503, and the file descriptors the
/// holds cost. Prints the registration rate; the operator then replaces
/// the pod and samples the probe.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "drives the deployed Station named by OUTPOST_AUDIT_ADDR"]
async fn register_many() {
    let addr = station();
    let count: u32 = std::env::var("OUTPOST_AUDIT_TERRITORIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let started = Instant::now();
    for seed in 1..=count {
        let (device, _, _) = territory_and_device_from(seed).await;
        let territory = device.territory_id().await.unwrap();
        device.register(addr.clone(), territory).await.unwrap();
        if seed % 50 == 0 {
            println!("{seed} registered after {:.1?}", started.elapsed());
        }
    }
    let elapsed = started.elapsed();
    println!(
        "registered {count} territories in {elapsed:.1?} ({:.0} ms each) — replace the pod and sample /readyz",
        elapsed.as_secs_f64() * 1000.0 / f64::from(count)
    );
}

/// Hold a live subscription while the operator replaces the pod: the
/// session must end with a close (not an idle reap), the same address must
/// serve again, and the Notice must still be there.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "drives the deployed Station named by OUTPOST_AUDIT_ADDR"]
async fn subscription_survives_replacement() {
    let addr = station();
    let (mut device, notice_id, _) = territory_and_device().await;
    let territory = device.territory_id().await.unwrap();
    device.register(addr.clone(), territory).await.unwrap();
    device.sync_with(addr.clone()).await.unwrap();

    let (_sub, mut events) = device.subscribe(addr.clone()).await.unwrap();
    println!("subscribed to {} — replace the pod now", addr.id);

    let started = Instant::now();
    let closed = timeout(operator_wait(), async {
        loop {
            match events.recv().await {
                Some(SubscriptionEvent::Closed) | None => break,
                Some(_) => continue,
            }
        }
    })
    .await;
    assert!(closed.is_ok(), "no close within the operator wait");
    let closed_after = started.elapsed();
    println!("session closed after {closed_after:.1?}");

    let deadline = Instant::now() + operator_wait();
    let resynced = loop {
        match device.sync_with(addr.clone()).await {
            Ok(()) => break Instant::now(),
            Err(e) if Instant::now() < deadline => {
                println!("not yet: {e}");
                sleep(Duration::from_secs(2)).await;
            }
            Err(e) => panic!("the address never served again: {e}"),
        }
    };
    println!(
        "resynced against the same address {:.1?} after the close",
        resynced.duration_since(started) - closed_after
    );
    assert!(station_holds(&addr, &device, notice_id).await);
}
