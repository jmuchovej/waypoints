//! Registration-driven, multi-Territory serving over real
//! QUIC. A Station opened empty with only a storage directory learns
//! Territories over the wire: a Vessel registers one (presenting its root),
//! the Station bootstraps a fresh hold, and it serves it. One Station homes
//! many Territories, and the set persists as the holds on disk — a restarted
//! Station reloads them and still serves, with the synced Notices intact.

use ed25519_dalek::SigningKey;
use outpost::{
    protocol::{
        ACLPayload, EpochID, InviteKey, KeyRoot, NoticeID, Permissions, RecordBody, RecordID,
    },
    station::Station,
    vessel::Vessel,
};
use rand::SeedableRng;
use std::sync::Arc;

fn station_key() -> SigningKey {
    SigningKey::from_bytes(&[100; 32])
}

/// A UDP port the kernel considers free right now — racy by nature, the
/// accepted cost of exercising a *fixed* bind.
fn free_udp_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
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

/// The deployed one-process Fleet: a routable orderer on an operator-chosen
/// port homes a Territory learned over the wire and orders a record for it.
/// Every socket it bound sits on that port, so the address a Service or a
/// firewall rule names is the address that serves.
#[tokio::test]
async fn a_routable_orderer_on_a_chosen_port_homes_and_orders() {
    let mut rng = rand::rngs::StdRng::from_seed([8; 32]);
    let dir = tempfile::tempdir().unwrap();
    let fleet_key = SigningKey::from_bytes(&[0xF1; 32]);
    let fleet_pub = fleet_key.verifying_key();
    let port = free_udp_port();

    let (owner_id, owner_dev) = {
        let r = KeyRoot::from_seed([3; 64]);
        (r.identity(), r.device(0))
    };
    let owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = owner.territory_id().await.unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap()[0].clone();

    let station = Arc::new(
        Station::hosting_orderer_routable_at(&station_key(), fleet_key, dir.path(), port)
            .await
            .unwrap(),
    );
    // No operator-advertised sockets: the address is the bound sockets, with
    // the wildcard rewritten to loopback — every one of them on `port`.
    let addr = station.advertised_addr();
    let ports: Vec<u16> = addr.ip_addrs().map(|s| s.port()).collect();
    assert!(!ports.is_empty());
    assert!(
        ports.iter().all(|p| *p == port),
        "every bound socket is on the configured port {port}: {ports:?}"
    );

    let serving = Arc::clone(&station);
    let serve_loop = tokio::spawn(async move { serving.serve_loop().await });

    // Learned over the wire, as in production: register, then order.
    owner.register(addr.clone(), territory).await.unwrap();
    assert!(station.serves(&territory));

    let invite_secret = SigningKey::from_bytes(&[9; 32]);
    let invite = RecordBody {
        prev: Some(RecordID::of(&root).unwrap()),
        author: owner_id.verifying_key(),
        payload: ACLPayload::Invite {
            invite_key: InviteKey::from_bytes(invite_secret.verifying_key().to_bytes()),
            permissions: Permissions::Writer,
        },
    }
    .sign(owner_id.signing_key())
    .unwrap();
    let proof = owner.submit_acl(addr, &invite).await.unwrap();
    proof
        .verify(&fleet_pub, RecordID::of(&invite).unwrap())
        .unwrap();
    assert_eq!(proof.position, 1);
    assert_eq!(
        station
            .replica_for(&territory)
            .unwrap()
            .acl_len()
            .await
            .unwrap(),
        2,
        "the orderer stored the ordered record after the root"
    );

    serve_loop.abort();
}
