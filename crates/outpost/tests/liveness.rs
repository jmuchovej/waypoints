//! Connection liveness: an idle-but-signalling subscription is retained,
//! and a Vessel that reconnects resumes from its cursor — receiving what
//! was authored while it was gone with no redundant resupply, reaching the
//! same state as a cold start.

use ed25519_dalek::SigningKey;
use outpost::{
    fabric::Liveness,
    protocol::{EpochID, KeyRoot, NoticeID},
    replica::Replica,
    station::Station,
    vessel::Vessel,
};
use rand::SeedableRng;
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

/// Short timings so reaping happens in test time, not 45s.
fn brisk() -> Liveness {
    Liveness {
        heartbeat: Duration::from_millis(50),
        idle_timeout: Duration::from_millis(300),
    }
}

/// The Station reaps a subscriber that stops signalling (heartbeat longer
/// than the Station idle timeout ⇒ it never keepalives, the vanished-Vessel
/// shape) while keeping a briskly-heartbeating one connected.
#[tokio::test]
async fn stale_subscription_reaped_idle_but_signalling_retained() {
    let mut rng = rand::rngs::StdRng::from_seed([3; 32]);
    let root = KeyRoot::from_seed([4; 64]);
    let identity = root.identity();

    let owner = Vessel::create_territory(&identity, &root.device(0), ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = owner.territory_id().await.unwrap();
    let acl = owner.replica().hold().acl_records().await.unwrap();

    let station = Arc::new(
        Station::new(
            &SigningKey::from_bytes(&[100; 32]),
            Replica::bootstrap(":memory:", &acl).await.unwrap(),
        )
        .await
        .unwrap()
        .with_liveness(brisk()),
    );
    let addr = station.endpoint().direct_addr();
    let serving = Arc::clone(&station);
    let serve_loop = tokio::spawn(async move { serving.serve_loop().await });

    // Live: brisk heartbeat, keeps the session warm.
    let live = Vessel::from_identity(
        &identity,
        &root.device(1),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    let (_live_sub, _live_events) = live.subscribe_with(addr.clone(), brisk()).await.unwrap();

    // Vanished: a heartbeat far longer than the Station idle timeout, so it
    // never sends a keepalive — indistinguishable from a Vessel that slept.
    let silent_liveness = Liveness {
        heartbeat: Duration::from_secs(3600),
        idle_timeout: Duration::from_secs(3600),
    };
    let gone = Vessel::from_identity(
        &identity,
        &root.device(2),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    let (_gone_sub, _gone_events) = gone
        .subscribe_with(addr.clone(), silent_liveness)
        .await
        .unwrap();

    let live_key = root.device(1).signing.verifying_key();
    let gone_key = root.device(2).signing.verifying_key();

    // The silent one is reaped; the brisk one stays connected.
    let mut reaped = false;
    for _ in 0..100 {
        let s = station.sightings_for(&territory);
        let gone_conn = s
            .iter()
            .find(|s| *s.device() == gone_key)
            .is_some_and(|s| s.connected);
        let live_conn = s
            .iter()
            .find(|s| *s.device() == live_key)
            .is_some_and(|s| s.connected);
        if !gone_conn && live_conn {
            reaped = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        reaped,
        "the vanished subscription is reaped while the signalling one is retained"
    );

    serve_loop.abort();
}

/// A Vessel whose session is severed reconnects and receives everything
/// authored while it was gone, reaching a cold-start Vessel state with no
/// redundant resupply.
#[tokio::test]
async fn reconnect_resumes_without_redundant_resupply() {
    let mut rng = rand::rngs::StdRng::from_seed([6; 32]);
    let root = KeyRoot::from_seed([7; 64]);
    let identity = root.identity();

    let owner = Vessel::create_territory(&identity, &root.device(0), ":memory:", &mut rng)
        .await
        .unwrap();
    let acl = owner.replica().hold().acl_records().await.unwrap();

    let station = Arc::new(
        Station::new(
            &SigningKey::from_bytes(&[100; 32]),
            Replica::bootstrap(":memory:", &acl).await.unwrap(),
        )
        .await
        .unwrap(),
    );
    let addr = station.endpoint().direct_addr();
    let serving = Arc::clone(&station);
    let serve_loop = tokio::spawn(async move { serving.serve_loop().await });

    let (owner_sub, _oe) = owner.subscribe(addr.clone()).await.unwrap();

    // A phone subscribes, then its session drops (handle + events dropped).
    let phone = Vessel::from_identity(
        &identity,
        &root.device(1),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    {
        let (sub, mut events) = phone.subscribe(addr.clone()).await.unwrap();
        drop(sub);
        // Drain until the worker signals Closed.
        let _ = timeout(WAIT, events.recv()).await;
    }

    // The owner authors two Notices while the phone is disconnected.
    for body in [b"while gone 1".as_slice(), b"while gone 2".as_slice()] {
        let n = owner
            .author(&mut rng, vec![], EpochID::ZERO, "waypoints.page.v1", body)
            .await
            .unwrap();
        owner_sub.push(n).await.unwrap();
    }
    let owner_ids: Vec<NoticeID> = owner
        .replica()
        .hold()
        .notices_after(-1)
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();

    // Reconnect: the initial reconcile resumes from what the phone already
    // holds, pulling only the missing Notices.
    let (_sub2, _e2) = phone.subscribe(addr.clone()).await.unwrap();
    let mut converged = false;
    for _ in 0..100 {
        let held = phone
            .replica()
            .hold()
            .notices_after(-1)
            .await
            .unwrap()
            .len();
        if held == owner_ids.len() {
            converged = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        converged,
        "reconnect must deliver the Notices authored while gone"
    );

    // Same state as a cold-start Vessel syncing once.
    let mut cold = Vessel::from_identity(
        &identity,
        &root.device(2),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    cold.sync_with(addr.clone()).await.unwrap();
    let cold_ids = cold.replica().hold().notices_after(-1).await.unwrap().len();
    assert_eq!(
        cold_ids,
        owner_ids.len(),
        "cold start reaches the same state"
    );

    drop(owner_sub);
    serve_loop.abort();
}
