//! End-to-end: vessel ↔ station ↔ vessel convergence over the whole stack
//! (design doc §12). One Vessel authors an *encrypted* Notice and pushes it
//! to a Station over real iroh QUIC; a second Vessel pulls from the Station
//! and decrypts it. This exercises every slice at once — the ACL fold and
//! key schedule, the validation loop, bedrock storage, the sync protocol,
//! and the iroh transport — through the public façade.
//!
//! The Station never holds a read key and never decrypts: it stores and
//! serves the opaque Notice, and the second Vessel is what unseals it. That
//! is the whole trust model (§3) demonstrated in one flow.

use ed25519_dalek::{Signer, SigningKey};
use outpost::protocol::acl::{
    ACLPayload, ACLRecord, InviteKey, Permissions, RecordBody, join_binding,
};
use outpost::protocol::crypto::{KemSecretKey, ReadKey};
use outpost::protocol::{EpochID, MemberID, NoticeID};
use outpost::replica::Replica;
use outpost::station::Station;
use outpost::vessel::Vessel;
use rand::SeedableRng;

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

/// An Owner + two Writer devices sharing one identity is unrealistic; here
/// each "vessel" is its own member for clarity. Owner (1) invites Writers
/// (2) and (3). Returns the ordered ACL log.
fn shared_acl() -> Vec<ACLRecord> {
    let owner = key(1);
    let sign = |a: &SigningKey, prev, p| {
        RecordBody {
            prev,
            author: a.verifying_key(),
            payload: p,
        }
        .sign(a)
        .unwrap()
    };
    // Track the running head via each record's id.
    let mut log = Vec::new();
    let root = sign(
        &owner,
        None,
        ACLPayload::Root {
            territory_seed: [0xA5; 32],
            owner_kem: KemSecretKey::from_seed([1; 32]).public_key(),
            epoch0_key_wrapped: vec![0xEE; 8],
        },
    );
    let mut head = RecordID_of(&root);
    log.push(root);

    for (writer_seed, invite_seed) in [(2u8, 9u8), (3u8, 10u8)] {
        let writer = key(writer_seed);
        let invite_secret = key(invite_seed);
        let invite = InviteKey::from_bytes(invite_secret.verifying_key().to_bytes());
        let inv = sign(
            &owner,
            Some(head),
            ACLPayload::Invite {
                invite_key: invite,
                permissions: Permissions::Writer,
            },
        );
        head = RecordID_of(&inv);
        log.push(inv);

        let writer_kem = KemSecretKey::from_seed([writer_seed; 32]).public_key();
        let proof =
            invite_secret.sign(&join_binding(&invite, &writer.verifying_key(), &writer_kem));
        let join = sign(
            &writer,
            Some(head),
            ACLPayload::Join {
                invite_key: invite,
                member_kem: writer_kem,
                invite_proof: proof,
            },
        );
        head = RecordID_of(&join);
        log.push(join);

        let approve = sign(
            &owner,
            Some(head),
            ACLPayload::Approve {
                member: MemberID::from(&writer.verifying_key()),
                permissions: Permissions::Writer,
                current_key_wrapped: vec![0xEE; 8],
            },
        );
        head = RecordID_of(&approve);
        log.push(approve);
    }
    log
}

#[allow(non_snake_case)]
fn RecordID_of(record: &ACLRecord) -> outpost::protocol::RecordID {
    outpost::protocol::RecordID::of(record).unwrap()
}

#[tokio::test]
async fn vessel_station_vessel_converges_over_quic() {
    let acl = shared_acl();
    // The Territory's epoch-0 read key. In production a Vessel unseals this
    // from its ACL sealed box; here we hand both Writers the same key
    // directly (key acquisition wiring is a later step).
    let read_key = ReadKey::from_bytes([0x42; 32]);

    // --- Station: stores and serves, never decrypts. ---
    let station_replica = Replica::bootstrap(":memory:", &acl).await.unwrap();
    let station = Station::new(&key(100), station_replica).await.unwrap();
    let station_addr = station.endpoint().direct_addr();

    // --- Vessel A: authors an encrypted Notice. ---
    let mut vessel_a = Vessel::new(
        key(2),
        KemSecretKey::from_seed([2; 32]),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    vessel_a.cache_read_key(EpochID::ZERO, read_key.clone());

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
        key(3),
        KemSecretKey::from_seed([3; 32]),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    vessel_b.cache_read_key(EpochID::ZERO, read_key);

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
