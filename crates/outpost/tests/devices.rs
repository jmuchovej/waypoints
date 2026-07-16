//! The device registry (§4: visibility + attribution + connection status)
//! over real QUIC. Two devices of one identity touch a Territory through a
//! Station; the owner asks the Station which devices have access and gets
//! back *verified* attribution — each sighting's cert checks out against its
//! identity, the two devices are distinct VesselIDs behind one MemberID.
//! Viewing is member-gated: a non-member identity that synced (being sighted
//! needs no membership) is still refused the listing.

use std::sync::Arc;

use outpost::protocol::MemberID;
use outpost::protocol::identity::KeyRoot;
use outpost::replica::Replica;
use outpost::station::Station;
use outpost::vessel::Vessel;
use rand::SeedableRng;

#[tokio::test]
async fn devices_are_attributed_per_identity_and_member_gated() {
    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);

    // One identity, two devices — the same mnemonic on a laptop and a phone.
    let root_key = KeyRoot::from_seed([1; 64]);
    let identity = root_key.identity();
    let laptop = root_key.device(0);
    let phone = root_key.device(1);

    // The owner creates the Territory on the "laptop".
    let mut owner_laptop = Vessel::create_territory(&identity, &laptop, ":memory:", &mut rng)
        .await
        .unwrap();
    let root = owner_laptop.replica().hold().acl_records().await.unwrap();

    // A Station seeded with the Territory.
    let station = Arc::new(
        Station::new(
            &ed25519_dalek::SigningKey::from_bytes(&[100; 32]),
            Replica::bootstrap(":memory:", &root).await.unwrap(),
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

    // Both devices sync; a stranger (different mnemonic, NOT a member) also
    // syncs — stations serve ciphertext without membership, so it is sighted.
    owner_laptop.sync_with(addr.clone()).await.unwrap();
    let mut owner_phone = Vessel::from_identity(
        &identity,
        &phone,
        Replica::bootstrap(":memory:", &root).await.unwrap(),
    )
    .await
    .unwrap();
    owner_phone.sync_with(addr.clone()).await.unwrap();

    let stranger_key = KeyRoot::from_seed([9; 64]);
    let mut stranger = Vessel::from_identity(
        &stranger_key.identity(),
        &stranger_key.device(0),
        Replica::bootstrap(":memory:", &root).await.unwrap(),
    )
    .await
    .unwrap();
    stranger.sync_with(addr.clone()).await.unwrap();

    // The owner lists devices: three sightings (laptop, phone, stranger's
    // device), every cert verified client-side, the owner's two devices
    // distinct VesselIDs behind the one MemberID.
    let sightings = owner_laptop.device_sightings(addr.clone()).await.unwrap();
    assert_eq!(sightings.len(), 3, "laptop + phone + stranger were sighted");
    let owner_member = identity.member_id();
    let owner_devices: Vec<_> = sightings
        .iter()
        .filter(|s| MemberID::from(&s.identity) == owner_member)
        .collect();
    assert_eq!(
        owner_devices.len(),
        2,
        "two devices behind the one identity"
    );
    assert_ne!(
        owner_devices[0].device(),
        owner_devices[1].device(),
        "distinct VesselIDs"
    );
    for s in &sightings {
        assert!(s.first_seen > 0 && s.last_seen >= s.first_seen);
        assert!(!s.connected, "one-shot sessions have all ended");
    }

    // A re-sight refreshes rather than duplicates.
    owner_phone.sync_with(addr.clone()).await.unwrap();
    let sightings = owner_laptop.device_sightings(addr.clone()).await.unwrap();
    assert_eq!(sightings.len(), 3, "re-sight refreshed, not duplicated");

    // Viewing is member-gated: the sighted-but-not-admitted stranger is
    // refused the listing.
    let refused = stranger.device_sightings(addr.clone()).await;
    assert!(
        matches!(refused, Err(outpost::vessel::VesselError::DevicesRefused)),
        "a non-member cannot list devices: {refused:?}"
    );

    serve_loop.abort();
}
