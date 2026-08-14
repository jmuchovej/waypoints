//! A Vessel opened from a mnemonic-derived identity authors and reads
//! its own encrypted history — proving the identity layer plugs into the
//! Vessel end to end, without hand-fed raw keys.

use outpost::{
    protocol::{
        ACLPayload, ACLRecord, EpochID, KeyRoot, NoticeID, ReadKey, RecordBody, seal_key_to_member,
    },
    replica::Replica,
    vessel::Vessel,
};
use rand::SeedableRng;

#[tokio::test]
async fn a_mnemonic_derived_vessel_authors_and_reads() {
    // A user with a mnemonic derives their identity + first device.
    let root = KeyRoot::from_seed([42; 64]);
    let identity = root.identity();
    let device = root.device(0);

    // Their Territory's Root seals the epoch-0 key to the identity's KEM key
    // — exactly what `Vessel::from_identity` unseals via refresh_keys.
    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);
    let read_key = ReadKey::from_bytes([0xA0; 32]);
    let sealed = seal_key_to_member(&mut rng, &identity.public_key(), &read_key).unwrap();
    let root_record: ACLRecord = RecordBody {
        prev: None,
        author: identity.verifying_key(),
        payload: ACLPayload::Root {
            territory_seed: [0xA5; 32],
            owner_kem: identity.public_key(),
            epoch0_key_wrapped: sealed,
        },
    }
    .sign(identity.signing_key())
    .unwrap();

    let vessel = Vessel::from_identity(
        &identity,
        &device,
        Replica::bootstrap(":memory:", &[root_record])
            .await
            .unwrap(),
    )
    .await
    .unwrap();

    // The identity is the member; the device cert is available for the
    // Phase-4 handshake; the endpoint is the device key (VesselID), distinct
    // from the identity key.
    assert_eq!(vessel.readable_epochs(), vec![EpochID::ZERO]);
    vessel
        .device_cert()
        .verify(&identity.verifying_key())
        .expect("the vessel presents the device cert");
    device
        .cert
        .verify(&identity.verifying_key())
        .expect("the device is certified by the identity");

    // Author an encrypted Notice under the identity, then read it back.
    let notice = vessel
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"mine",
        )
        .await
        .unwrap();
    assert_eq!(
        notice.parse_and_verify().unwrap().header.author,
        identity.verifying_key(),
        "Notices are authored by the identity key, not the device key",
    );
    let stored = vessel
        .replica()
        .hold()
        .get_notice(NoticeID::of(&notice).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(vessel.decrypt(&stored.notice).unwrap(), b"mine");
}
