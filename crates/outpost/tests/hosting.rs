//! Registration-driven, multi-Territory serving over real
//! QUIC. A Station opened empty with only a storage directory learns
//! Territories over the wire: a Vessel registers one (presenting its root),
//! the Station bootstraps a fresh hold, and it serves it. One Station homes
//! many Territories, and the set persists as the holds on disk — a restarted
//! Station reloads them and still serves, with the synced Notices intact.

use std::sync::Arc;

use ed25519_dalek::SigningKey;
use outpost::protocol::identity::KeyRoot;
use outpost::protocol::{EpochID, NoticeID};
use outpost::station::Station;
use outpost::vessel::Vessel;
use rand::SeedableRng;

fn station_key() -> SigningKey {
    SigningKey::from_bytes(&[100; 32])
}

#[tokio::test]
async fn a_station_homes_many_territories_learned_over_the_wire() {
    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);
    let dir = tempfile::tempdir().unwrap();

    // Two independent owners, two independent Territories.
    let (id_a, dev_a) = {
        let r = KeyRoot::from_seed([1; 64]);
        (r.identity(), r.device(0))
    };
    let (id_b, dev_b) = {
        let r = KeyRoot::from_seed([2; 64]);
        (r.identity(), r.device(0))
    };
    let mut owner_a = Vessel::create_territory(&id_a, &dev_a, ":memory:", &mut rng)
        .await
        .unwrap();
    let mut owner_b = Vessel::create_territory(&id_b, &dev_b, ":memory:", &mut rng)
        .await
        .unwrap();
    let territory_a = owner_a.territory_id().await.unwrap();
    let territory_b = owner_b.territory_id().await.unwrap();

    // Owner A authors an encrypted Notice to push through the registered
    // Territory, so the sync moves real data, not just the root.
    let notice = owner_a
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"a's secret",
        )
        .await
        .unwrap();
    let notice_id = NoticeID::of(&notice).unwrap();

    // The Station starts EMPTY — no root handed to it, just a storage dir.
    let station = Arc::new(Station::hosting(&station_key(), dir.path()).await.unwrap());
    let addr = station.endpoint().direct_addr();
    assert!(!station.serves(&territory_a), "starts serving nothing");

    let serving = station.clone();
    let serve_loop = tokio::spawn(async move {
        loop {
            let _ = serving.serve_once().await;
        }
    });

    // Each owner registers its Territory (the Station bootstraps a hold from
    // the presented root) and then syncs.
    owner_a.register(addr.clone(), territory_a).await.unwrap();
    owner_a.sync_with(addr.clone()).await.unwrap();
    owner_b.register(addr.clone(), territory_b).await.unwrap();
    owner_b.sync_with(addr.clone()).await.unwrap();

    // The one Station now homes both Territories, and holds A's pushed Notice
    // in A's hold — content-blind, but present.
    assert!(station.serves(&territory_a));
    assert!(station.serves(&territory_b));
    assert!(
        station
            .replica_for(&territory_a)
            .unwrap()
            .hold()
            .get_notice(notice_id)
            .await
            .unwrap()
            .is_some(),
        "A's Notice reached the Station's hold for Territory A"
    );

    serve_loop.abort();
    drop(station);

    // Restart from the same storage dir: the holds on disk are the registry.
    let restarted = Station::hosting(&station_key(), dir.path()).await.unwrap();
    assert!(restarted.serves(&territory_a), "reloaded Territory A");
    assert!(restarted.serves(&territory_b), "reloaded Territory B");
    assert!(
        restarted
            .replica_for(&territory_a)
            .unwrap()
            .hold()
            .get_notice(notice_id)
            .await
            .unwrap()
            .is_some(),
        "A's Notice survived the restart"
    );
}
