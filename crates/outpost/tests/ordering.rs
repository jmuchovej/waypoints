//! ACL records are ordered by a fleet orderer and carry a fleet-key proof
//! that peers verify (full ordering). A vessel submits a membership
//! record to the orderer Station over real QUIC, receives the countersigned
//! proof, and the ordered record propagates to a *fleeted* peer that
//! verifies it — while a record with a forged proof is rejected.

use ed25519_dalek::SigningKey;
use outpost::{
    protocol::{
        ACLPayload, DeviceKey, Identity, InviteKey, KeyRoot, OrderProof, OrderedRecord,
        Permissions, ReadKey, RecordBody, RecordID,
    },
    replica::Replica,
    station::Station,
    vessel::Vessel,
};
use rand::SeedableRng;

fn identity(seed: u8) -> (Identity, DeviceKey) {
    let root = KeyRoot::from_seed([seed; 64]);
    (root.identity(), root.device(0))
}

#[tokio::test]
async fn a_submitted_record_is_ordered_proven_and_accepted_by_a_fleeted_peer() {
    let mut rng = rand::rngs::StdRng::from_seed([5; 32]);
    let fleet_key = SigningKey::from_bytes(&[0xF1; 32]);
    let fleet_pub = fleet_key.verifying_key();
    let (owner_id, owner_dev) = identity(1);

    // Owner creates the Territory; its Root is the shared seed.
    let owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap()[0].clone();

    // The orderer Station holds the same Root and the fleet signing key.
    let station = Station::orderer(
        &SigningKey::from_bytes(&[100; 32]),
        fleet_key.clone(),
        Replica::bootstrap(":memory:", std::slice::from_ref(&root))
            .await
            .unwrap(),
    )
    .await
    .unwrap();
    let station_addr = station.endpoint().direct_addr();

    // Owner authors an Invite chained on the Root (not yet ordered).
    let invite_secret = SigningKey::from_bytes(&[9; 32]);
    let invite_key = InviteKey::from_bytes(invite_secret.verifying_key().to_bytes());
    let invite = RecordBody {
        prev: Some(RecordID::of(&root).unwrap()),
        author: owner_id.verifying_key(),
        payload: ACLPayload::Invite {
            invite_key,
            permissions: Permissions::Writer,
        },
    }
    .sign(owner_id.signing_key())
    .unwrap();
    let invite_id = RecordID::of(&invite).unwrap();

    // Submit it to the orderer over QUIC; it comes back countersigned.
    let serve = tokio::spawn(async move {
        station.serve_once().await.unwrap();
        station
    });
    let proof = owner.submit_acl(station_addr, &invite).await.unwrap();
    let station = serve.await.unwrap();

    // The proof verifies for this record against the fleet key, and the
    // orderer has stored the ordered record at position 1.
    proof.verify(&fleet_pub, invite_id).unwrap();
    assert_eq!(proof.position, 1);
    assert_eq!(station.replica().acl_len().await.unwrap(), 2);

    // A *fleeted* peer (bootstrapped with only the Root + the fleet key)
    // ingests the ordered record and accepts it because the proof verifies.
    let peer = Replica::bootstrap_with_fleet(":memory:", std::slice::from_ref(&root), fleet_pub)
        .await
        .unwrap();
    peer.ingest_ordered(vec![OrderedRecord {
        record: invite.clone(),
        proof: Some(proof),
    }])
    .await
    .unwrap();
    assert_eq!(
        peer.acl_len().await.unwrap(),
        2,
        "the proven record is accepted"
    );

    // A forged proof (wrong fleet key) is rejected: the fleeted peer refuses
    // to grow its log.
    let forged = OrderProof::sign(&SigningKey::from_bytes(&[0xEE; 32]), invite_id, 1);
    let peer2 = Replica::bootstrap_with_fleet(":memory:", &[root], fleet_pub)
        .await
        .unwrap();
    peer2
        .ingest_ordered(vec![OrderedRecord {
            record: invite,
            proof: Some(forged),
        }])
        .await
        .unwrap();
    assert_eq!(
        peer2.acl_len().await.unwrap(),
        1,
        "a record with a forged order proof is rejected",
    );
}

#[tokio::test]
async fn the_root_needs_no_proof_but_others_do() {
    // On a fleeted replica, the Root (no proof) is accepted, but a non-root
    // record without a proof is refused — that is what "full ordering" buys.
    let mut rng = rand::rngs::StdRng::from_seed([6; 32]);
    let fleet = SigningKey::from_bytes(&[0xF1; 32]).verifying_key();
    let (owner_id, owner_dev) = identity(1);
    let _ = ReadKey::from_bytes([0; 32]); // (keys handled by create_territory)

    let owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap()[0].clone();

    // Root alone bootstraps a fleeted replica fine (Root is exempt).
    let peer = Replica::bootstrap_with_fleet(":memory:", std::slice::from_ref(&root), fleet)
        .await
        .unwrap();
    assert_eq!(peer.acl_len().await.unwrap(), 1);

    // A non-root record with no proof is rejected on a fleeted peer.
    let unproven = RecordBody {
        prev: Some(RecordID::of(&root).unwrap()),
        author: owner_id.verifying_key(),
        payload: ACLPayload::RevokeInvite {
            invite_key: InviteKey::from_bytes([7; 32]),
        },
    }
    .sign(owner_id.signing_key())
    .unwrap();
    peer.ingest_ordered(vec![OrderedRecord {
        record: unproven,
        proof: None,
    }])
    .await
    .unwrap();
    assert_eq!(
        peer.acl_len().await.unwrap(),
        1,
        "an unproven non-root record is refused",
    );
}
