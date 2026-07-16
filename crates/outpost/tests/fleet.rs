//! Fleet enforcement + territory registration (§10.2, §13) over real QUIC.
//! A Vessel holding a signed FleetConfig registers its Territory with a
//! rostered Station and syncs; a Station not in the roster is refused; the
//! identity handshake proves which identity is behind the device.

use ed25519_dalek::SigningKey;
use outpost::protocol::identity::KeyRoot;
use outpost::replica::Replica;
use outpost::station::Station;
use outpost::station::fleet::{FleetId, RosterBody, StationEntry, StationRoles};
use outpost::vessel::Vessel;
use rand::SeedableRng;

fn identity(
    seed: u8,
) -> (
    outpost::protocol::identity::Identity,
    outpost::protocol::identity::DeviceKey,
) {
    let root = KeyRoot::from_seed([seed; 64]);
    (root.identity(), root.device(0))
}

/// A signed FleetConfig listing `station_key` as a storage station.
fn fleet_config(
    fleet_key: &SigningKey,
    station_key: ed25519_dalek::VerifyingKey,
) -> outpost::station::fleet::FleetConfig {
    RosterBody {
        fleet: FleetId(fleet_key.verifying_key()),
        revision: 1,
        stations: vec![StationEntry {
            peer_key: station_key,
            addrs: vec![],
            roles: StationRoles {
                storage: true,
                orderer: false,
                blob: false,
            },
        }],
    }
    .sign(fleet_key)
}

#[tokio::test]
async fn a_vessel_registers_with_a_rostered_station_and_syncs() {
    let mut rng = rand::rngs::StdRng::from_seed([4; 32]);
    let fleet_key = SigningKey::from_bytes(&[0xF1; 32]);
    let station_secret = SigningKey::from_bytes(&[100; 32]);
    let config = fleet_config(&fleet_key, station_secret.verifying_key());

    let (owner_id, owner_dev) = identity(1);
    let owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = owner.territory_id().await.unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap();

    // The station holds the Territory's Root and is in the roster.
    let station = Station::new(
        &station_secret,
        Replica::bootstrap(":memory:", &root).await.unwrap(),
    )
    .await
    .unwrap();
    let station_addr = station.endpoint().direct_addr();

    // Register the Territory: the Station verifies the identity handshake and
    // records it as served.
    let serve = tokio::spawn(async move {
        station.serve_once().await.unwrap();
        station
    });
    owner
        .register_with_fleet(&config, station_addr.clone(), territory)
        .await
        .unwrap();
    let station = serve.await.unwrap();
    assert!(
        station.serves(&territory),
        "the Station now holds the Territory"
    );

    // And a fleet-verified sync succeeds against the rostered Station.
    let mut owner = owner;
    let serve = tokio::spawn(async move { station.serve_once().await.unwrap() });
    owner.sync_with_fleet(&config, station_addr).await.unwrap();
    serve.await.unwrap();
}

#[tokio::test]
async fn a_station_not_in_the_roster_is_refused() {
    let mut rng = rand::rngs::StdRng::from_seed([8; 32]);
    let fleet_key = SigningKey::from_bytes(&[0xF1; 32]);
    // The config lists a DIFFERENT station key than the one we will dial.
    let config = fleet_config(
        &fleet_key,
        SigningKey::from_bytes(&[200; 32]).verifying_key(),
    );

    let (owner_id, owner_dev) = identity(1);
    let mut owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap();

    // The station we actually dial (key 100) is not in the roster.
    let station = Station::new(
        &SigningKey::from_bytes(&[100; 32]),
        Replica::bootstrap(":memory:", &root).await.unwrap(),
    )
    .await
    .unwrap();
    let station_addr = station.endpoint().direct_addr();

    let serve = tokio::spawn(async move {
        // The station may accept then see the connection drop; ignore.
        let _ = station.serve_once().await;
    });
    let result = owner.sync_with_fleet(&config, station_addr).await;
    let _ = serve.await;
    assert!(
        matches!(result, Err(outpost::vessel::VesselError::UntrustedStation)),
        "syncing with an unrostered Station is refused, got {result:?}",
    );
}
