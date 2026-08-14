//! End-to-end: vessel ↔ station ↔ vessel convergence over the whole stack
//! One Vessel authors an *encrypted* Notice and pushes it
//! to a Station over real iroh QUIC; a second Vessel pulls from the Station
//! and decrypts it. This exercises every slice at once — the ACL fold and
//! key schedule, read-key acquisition from the log, the validation loop,
//! bedrock storage, the sync protocol, and the iroh transport — through the
//! public façade.
//!
//! The Station never holds a read key and never decrypts: it stores and
//! serves the opaque Notice, and the second Vessel is what unseals it. Both
//! Vessels obtain their key the real way — [`Vessel::refresh_keys`] over the
//! ACL log — not by being handed one.
//!
//! [`Vessel::refresh_keys`]: outpost::vessel::Vessel::refresh_keys

mod common;

use common::{TerritoryBuilder, kem, signing};
use outpost::{
    protocol::{EpochID, NoticeID, ReadKey},
    replica::Replica,
    station::Station,
    vessel::Vessel,
};
use rand::SeedableRng;

#[tokio::test]
async fn vessel_station_vessel_converges_over_quic() {
    // A Territory owned by member 1, with Writers 2 and 3, epoch-0 key sealed
    // to each via real ACL sealed boxes.
    let k0 = ReadKey::from_bytes([0x42; 32]);
    let mut t = TerritoryBuilder::new(1, &k0);
    t.admit(1, 2, &k0);
    t.admit(1, 3, &k0);
    let acl = t.into_records();

    // --- Station: stores and serves, never decrypts. Its identity (100) is
    //     not a member, so no sealed box is ever addressed to it. ---
    let station_replica = Replica::bootstrap(":memory:", &acl).await.unwrap();
    let station = Station::new(&signing(100), station_replica).await.unwrap();
    let station_addr = station.endpoint().direct_addr();

    // --- Vessel A: derives its key from the log, authors an encrypted Notice. ---
    // `Vessel::new` auto-derives keys from the bootstrap ACL — no explicit
    // refresh needed.
    let mut vessel_a = Vessel::new(
        signing(2),
        kem(2),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();

    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);
    let notice = vessel_a
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"the secret contents",
        )
        .await
        .unwrap();
    let notice_id = NoticeID::of(&notice).unwrap();

    // --- Vessel A → Station: push. ---
    let station_serve = tokio::spawn(async move {
        station.serve_once().await.unwrap();
        station
    });
    vessel_a.sync_with(station_addr.clone()).await.unwrap();
    let station = station_serve.await.unwrap();

    // The Station now holds the opaque Notice — but cannot read it.
    assert!(
        station
            .replica()
            .hold()
            .get_notice(notice_id)
            .await
            .unwrap()
            .is_some()
    );

    // --- Vessel B ← Station: pull, then decrypt. ---
    let mut vessel_b = Vessel::new(
        signing(3),
        kem(3),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();

    let station_serve = tokio::spawn(async move {
        station.serve_once().await.unwrap();
    });
    vessel_b.sync_with(station_addr).await.unwrap();
    station_serve.await.unwrap();

    // Vessel B converged, and — unlike the Station — can decrypt.
    let stored = vessel_b
        .replica()
        .hold()
        .get_notice(notice_id)
        .await
        .unwrap()
        .expect("vessel B pulled the notice");
    let plaintext = vessel_b.decrypt(&stored.notice).unwrap();
    assert_eq!(plaintext, b"the secret contents");
}
