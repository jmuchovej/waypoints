//! The app-facing pairing primitives over real QUIC: a Station can be
//! probed for liveness (§9: does it serve this Territory?), and a brand-new
//! device that knows only the TerritoryID + Station address can fetch the
//! ACL log to bootstrap a local replica — the second-device onboarding step
//! (§4: same mnemonic, fresh device index, no admission required).

use outpost::{
    protocol::KeyRoot,
    replica::Replica,
    station::Station,
    vessel::{PairingPayload, Vessel, fetch_acl_log, reach},
};
use rand::SeedableRng;
use std::sync::Arc;

#[tokio::test]
async fn a_station_is_probeable_and_a_fresh_device_bootstraps_from_it() {
    let mut rng = rand::rngs::StdRng::from_seed([21; 32]);

    // First device: identity + Territory.
    let root_key = KeyRoot::from_seed([3; 64]);
    let identity = root_key.identity();
    let laptop = root_key.device(0);
    let mut owner = Vessel::create_territory(&identity, &laptop, ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = owner.territory_id().await.unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap();

    // An empty hosting Station: it does not serve the Territory yet.
    let dir = tempfile::tempdir().unwrap();
    let station = Arc::new(
        Station::hosting(
            &ed25519_dalek::SigningKey::from_bytes(&[42; 32]),
            dir.path(),
        )
        .await
        .unwrap(),
    );
    let addr = station.endpoint().direct_addr();
    let serving = station.clone();
    let serve_loop = tokio::spawn(async move {
        loop {
            let _ = serving.serve_once().await;
        }
    });

    // Probe before registration: reachable, but the Territory is not served.
    assert!(
        !owner.probe(addr.clone()).await.unwrap(),
        "an unregistered Territory is not served"
    );

    // Register + sync, then probe again: served.
    owner.register(addr.clone(), territory).await.unwrap();
    owner.sync_with(addr.clone()).await.unwrap();
    assert!(
        owner.probe(addr.clone()).await.unwrap(),
        "a registered Territory is served"
    );

    // A pairing payload round-trips through its QR/clipboard encoding and
    // hands a fresh device everything it needs.
    let payload = PairingPayload::new("not a real phrase".into(), territory, addr.clone());
    let decoded = PairingPayload::decode(&payload.encode().unwrap()).unwrap();
    assert_eq!(decoded.territory_id().unwrap(), territory);
    assert!(matches!(
        PairingPayload::decode("{\"v\":1,\"kind\":\"nonsense\"}"),
        Err(outpost::vessel::PairingError::Encoding(_)
            | outpost::vessel::PairingError::Unrecognized)
    ));

    // A brand-new device (same mnemonic, fresh random hardened index) holds
    // no replica — it can still reach the Station, then fetches the ACL log
    // and bootstraps one.
    let phone = root_key.random_device(&mut rng);
    assert!(phone.index >= 1 && phone.index < 0x8000_0000);
    reach(&phone, addr.clone()).await.unwrap();
    let log = fetch_acl_log(&identity, &phone, addr.clone(), territory)
        .await
        .unwrap();
    assert_eq!(log, root, "the fetched log is the Territory's ACL log");

    let replica = Replica::bootstrap(":memory:", &log).await.unwrap();
    let mut phone_vessel = Vessel::from_identity(&identity, &phone, replica)
        .await
        .unwrap();
    phone_vessel.sync_with(addr.clone()).await.unwrap();
    assert_eq!(
        phone_vessel.territory_id().await.unwrap(),
        territory,
        "the fresh device converged on the same Territory"
    );

    serve_loop.abort();
}
