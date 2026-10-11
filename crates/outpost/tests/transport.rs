//! Routable transport posture: a Station reachable at an *advertised*
//! address, and a dial-only Vessel that needs no inbound reachability —
//! live `NoticePush` travels back over the session the Vessel opened.
//!
//! In-test the advertised sockets are the loopback-rewritten bound ports
//! (a stand-in for the operator public `host:port`); what matters is that
//! the Vessel dials only what the Station *advertises*, never its bound
//! socket list.

use ed25519_dalek::SigningKey;
use outpost::{
    protocol::{EpochID, KeyRoot, NoticeID},
    replica::Replica,
    station::Station,
    vessel::{SubscriptionEvent, Vessel},
};
use rand::SeedableRng;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn dial_only_vessel_syncs_and_receives_push_at_an_advertised_address() {
    let mut rng = rand::rngs::StdRng::from_seed([9; 32]);
    let root_key = KeyRoot::from_seed([2; 64]);
    let identity = root_key.identity();

    let laptop = Vessel::create_territory(&identity, &root_key.device(0), ":memory:", &mut rng)
        .await
        .unwrap();
    let acl = laptop.replica().hold().acl_records().await.unwrap();

    // Routable Station, advertising operator-configured sockets.
    let station = Station::new_routable(
        &SigningKey::from_bytes(&[100; 32]),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    let public: Vec<SocketAddr> = station
        .endpoint()
        .direct_addr()
        .ip_addrs()
        .copied()
        .collect();
    let station = Arc::new(station.with_advertised(public.clone()));
    let addr = station.advertised_addr();
    assert_eq!(
        addr.ip_addrs().copied().collect::<Vec<_>>(),
        public,
        "the advertised address must carry exactly the configured sockets"
    );

    let serving = Arc::clone(&station);
    let serve_loop = tokio::spawn(async move { serving.serve_loop().await });

    // A second device: routable bind, dial-only. It subscribes — the
    // session *it* opened is the only channel the Station ever uses to
    // reach it.
    let mut phone = Vessel::from_identity_routable(
        &identity,
        &root_key.device(1),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    let (_phone_sub, mut phone_events) = phone.subscribe(addr.clone()).await.unwrap();

    // Live push: the laptop authors; the phone receives over its own session.
    let (laptop_sub, _laptop_events) = laptop.subscribe(addr.clone()).await.unwrap();
    let notice = laptop
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"over the advertised address",
        )
        .await
        .unwrap();
    let notice_id = NoticeID::of(&notice).unwrap();
    laptop_sub.push(notice).await.unwrap();

    let event = timeout(WAIT, phone_events.recv())
        .await
        .expect("doorbell within the wait")
        .expect("channel open");
    assert!(matches!(event, SubscriptionEvent::Notices { .. }));
    assert!(
        phone
            .replica()
            .hold()
            .get_notice(notice_id)
            .await
            .unwrap()
            .is_some(),
        "NoticePush must land over the Vessel-opened session"
    );

    // The ordinary one-shot sync also works at the advertised address.
    phone.sync_with(addr.clone()).await.unwrap();

    drop(laptop_sub);
    serve_loop.abort();
}

/// The Station is accept-only: `station.rs` must contain no outbound
/// `.connect(` call — `NoticePush` rides sessions Vessels opened. The
/// transport work must never regress this, so the inspection is pinned as
/// a test.
#[test]
fn station_source_opens_no_outbound_connections() {
    let src = include_str!("../src/station.rs");
    assert_eq!(
        src.matches(".connect(").count(),
        0,
        "station.rs must not initiate connections to Vessels"
    );
}
