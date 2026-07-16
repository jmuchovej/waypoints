//! Content-blindness of the Station (§3 rule 4), proven two ways.
//!
//! The **behavioral** test is the real one: it hands a Station the encrypted
//! Notice *and* the sealed read-key material from the ACL log, and shows the
//! Station cannot recover the plaintext — because a Station is not a
//! Territory member, so no ACL sealed box ever sealed the read key to it.
//! That is the cryptographic reason a Station stores-but-cannot-read: not a
//! missing method, but a key it can never obtain. A member holding the KEM
//! secret the box was sealed to recovers the plaintext from the very bytes
//! the Station serves.
//!
//! The **lexical** guard is a cheaper backstop: the content-blind modules
//! (`station`, `replica`) must name no key-material or decryption symbol,
//! while `vessel` — the one place that capability legitimately lives — must.
//! It catches the realistic code regression (importing `crypto` into the
//! station module) that the behavioral test would not, since a hand-rolled
//! decryption there would still fail for lack of a key but ought never to
//! exist. Neither test is defeat-proof against a determined edit; paired
//! with review they restore most of what the old crate boundary gave.

use ed25519_dalek::{Signer, SigningKey};
use outpost::protocol::acl::{ACLPayload, ACLRecord, RecordBody};
use outpost::protocol::crypto::{
    KemSecretKey, ReadKey, open_payload, open_sealed_key, seal_key_to_member,
};
use outpost::protocol::{EpochID, NoticeID};
use outpost::replica::Replica;
use outpost::vessel::Vessel;
use rand::SeedableRng;

/// A single-member Territory whose Root ACL record carries a **real** sealed
/// box: the epoch-0 read key sealed to `member`'s KEM key (§7.1). This is
/// what a member unseals to read; it is also what a Station provably cannot.
fn sealed_root(
    member: &SigningKey,
    member_kem: &KemSecretKey,
    sealed_key: &[u8],
) -> Vec<ACLRecord> {
    let root = RecordBody {
        prev: None,
        author: member.verifying_key(),
        payload: ACLPayload::Root {
            territory_seed: [0xA5; 32],
            owner_kem: member_kem.public_key(),
            epoch0_key_wrapped: sealed_key.to_vec(),
        },
    }
    .sign(member)
    .unwrap();
    vec![root]
}

#[tokio::test]
async fn station_holds_ciphertext_but_cannot_obtain_the_key() {
    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);
    let plaintext = b"the secret contents";
    let read_key = ReadKey::from_bytes([0x42; 32]);

    // The Territory's owner (also our authoring member) and their KEM key.
    let member = SigningKey::from_bytes(&[2; 32]);
    let member_kem = KemSecretKey::from_seed([2; 32]);

    // The Root ACL record seals epoch-0 to the member — a real X-Wing box.
    let sealed_box = seal_key_to_member(&mut rng, &member_kem.public_key(), &read_key).unwrap();
    let acl = sealed_root(&member, &member_kem, &sealed_box);

    // A member Vessel derives the key from the ACL log at construction (the
    // real path — `Vessel::new` unseals the Root box it is the owner of),
    // authors an encrypted Notice, and it reaches the Station's store.
    let vessel = Vessel::new(
        member.clone(),
        KemSecretKey::from_seed([2; 32]),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    let notice = vessel
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            plaintext,
        )
        .await
        .unwrap();
    let id = NoticeID::of(&notice).unwrap();

    // The Station's storage receives the Notice (as it would over sync).
    let station = Replica::bootstrap(":memory:", &acl).await.unwrap();
    station.ingest(vec![notice]).await.unwrap();

    // 1. A member CAN obtain the read key from the ACL's sealed box.
    assert_eq!(
        open_sealed_key(&member_kem, &sealed_box).unwrap(),
        read_key,
        "the member the box was sealed to recovers the key"
    );

    // 2. Hand a Station the same sealed box: it FAILS. A Station is not a
    //    member, so its (identity-derived) KEM secret is not the one the box
    //    was sealed to — it can never obtain the key. This is the crux.
    let station_kem = KemSecretKey::from_seed([200; 32]);
    assert!(
        open_sealed_key(&station_kem, &sealed_box).is_err(),
        "a Station cannot unseal a read key sealed to a member"
    );

    // 3. What the Station actually holds is ciphertext, not the plaintext.
    let stored = station.hold().get_notice(id).await.unwrap().unwrap();
    let envelope = stored.notice.parse_and_verify().unwrap();
    assert_ne!(
        envelope.payload.as_slice(),
        plaintext.as_slice(),
        "the Station stores the sealed payload, never the plaintext"
    );

    // 4. And the plaintext falls out of those very bytes for whoever holds
    //    the key a member (not the Station) could get — proving the Station
    //    serves the content while lacking the means to read it.
    let key = open_sealed_key(&member_kem, &sealed_box).unwrap();
    assert_eq!(
        open_payload(&key, &envelope.header, &envelope.payload).unwrap(),
        plaintext,
        "a keyholder decrypts the same bytes the Station only stores"
    );
}

// ---- lexical backstop -----------------------------------------------------

/// Symbols that name read-key material or a decryption operation.
const DECRYPTION_SYMBOLS: &[&str] = &[
    "ReadKey",
    "KemSecretKey",
    "open_payload",
    "open_sealed_key",
    "unwrap_prev_key",
    "decrypt",
];

/// Drop everything from `//` to end of line, so doc/prose mentions (a
/// Station's docs say it "never decrypts") don't count — only real code.
fn strip_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The forbidden symbols that appear in `source`'s code. Returned needles
/// come from `symbols`, not from `source`.
fn mentions(source: &str, symbols: &[&'static str]) -> Vec<&'static str> {
    let code = strip_comments(source);
    symbols
        .iter()
        .filter(|s| code.contains(*s))
        .copied()
        .collect()
}

const STATION_SRC: &str = include_str!("../src/station.rs");
const REPLICA_SRC: &str = include_str!("../src/replica.rs");
const VESSEL_SRC: &str = include_str!("../src/vessel.rs");

#[test]
fn station_and_replica_name_no_decryption() {
    for (name, src) in [("station", STATION_SRC), ("replica", REPLICA_SRC)] {
        let found = mentions(src, DECRYPTION_SYMBOLS);
        assert!(
            found.is_empty(),
            "the {name} module must hold no key material and invoke no \
             decryption, but its code names: {found:?}. If this is a real \
             design change, update the design doc — don't just delete this."
        );
    }
}

#[test]
fn decryption_lives_only_in_the_vessel() {
    let found = mentions(VESSEL_SRC, DECRYPTION_SYMBOLS);
    assert!(
        found.contains(&"open_payload") && found.contains(&"ReadKey"),
        "decryption should live in the vessel module; found: {found:?}"
    );
}
