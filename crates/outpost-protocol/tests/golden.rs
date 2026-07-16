//! Golden byte vectors pinning the wire format (§5.2). Ed25519 signing is
//! deterministic (RFC 8032) and postcard is canonical for a given value, so
//! every byte below is reproducible. If one of these assertions breaks, the
//! wire format changed: that is either a deliberate [`ENVELOPE_VERSION`]
//! bump with migration thinking behind it, or a bug — never a test to
//! silently update.

use ed25519_dalek::SigningKey;
use outpost_protocol::{
    ENVELOPE_VERSION, Envelope, EnvelopeHeader, EpochID, NoticeID, RecordID, SignedNotice,
};
use pretty_assertions::assert_eq;

// The author key below is RFC 8032's test-1 seed, so the `20d75a98…511a`
// run inside these vectors is the RFC's public key — an external
// cross-check that the Ed25519 layer is the standard construction.
const ENVELOPE_HEX: &str = "01021111111111111111111111111111111111111111111111111111111111111111222222222222222222222222222222222222222222222222222222222222222233333333333333333333333333333333333333333333333333333333333333330420d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a55555555555555555555555555555555555555555555555555555555555555550011776179706f696e74732e706167652e763180b7b4870d04deadbeef";
const SIGNED_NOTICE_HEX: &str = "c10101021111111111111111111111111111111111111111111111111111111111111111222222222222222222222222222222222222222222222222222222222222222233333333333333333333333333333333333333333333333333333333333333330420d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a55555555555555555555555555555555555555555555555555555555555555550011776179706f696e74732e706167652e763180b7b4870d04deadbeef40be5b91636df1ad9f945cc48f95d7f0717844624d13c341af7a9fe13b012af5cc9c6a93bfe2e1c77440d84545bd659887fe9aa203fd1a3f3fdcf764ed206ebe03";
const NOTICE_ID_HEX: &str = "6fa405256f603b12ca75a88f0a8617a0c61cb9857865b67a46fc99aae72d8d35";

fn vector_key() -> SigningKey {
    SigningKey::from_bytes(&[
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ])
}

fn vector_envelope() -> Envelope {
    Envelope {
        header: EnvelopeHeader {
            parents: vec![
                NoticeID::from_bytes([0x11; 32]),
                NoticeID::from_bytes([0x22; 32]),
            ],
            acl_head: RecordID::from_bytes([0x33; 32]),
            epoch: EpochID::from(4),
            author: vector_key().verifying_key(),
            snapshot_base: NoticeID::from_bytes([0x55; 32]),
            is_snapshot: false,
            payload_type: "waypoints.page.v1".into(),
            timestamp: 1_752_600_000,
        },
        payload: vec![0xde, 0xad, 0xbe, 0xef],
    }
}

#[test]
fn envelope_encoding_is_pinned() {
    let bytes = vector_envelope().encode().unwrap();
    assert_eq!(bytes[0], ENVELOPE_VERSION);
    assert_eq!(hex::encode(&bytes), ENVELOPE_HEX);
}

#[test]
fn signed_notice_encoding_is_pinned() {
    let notice = SignedNotice::sign(&vector_envelope(), &vector_key()).unwrap();
    assert_eq!(hex::encode(notice.encode().unwrap()), SIGNED_NOTICE_HEX);
}

#[test]
fn notice_id_is_pinned() {
    let notice = SignedNotice::sign(&vector_envelope(), &vector_key()).unwrap();
    assert_eq!(NoticeID::of(&notice).unwrap().to_string(), NOTICE_ID_HEX);
}

#[test]
fn pinned_bytes_parse_and_verify() {
    let notice = SignedNotice::decode(&hex::decode(SIGNED_NOTICE_HEX).unwrap()).unwrap();
    let envelope = notice.parse_and_verify().unwrap();
    assert_eq!(envelope, vector_envelope());
}

// A fully deterministic root ACL record: Ed25519 signing is deterministic
// and X-Wing key derivation from a fixed seed is specified expansion, so
// this RecordID pins the record encoding, the signature, and the KEM key
// derivation all at once.
const ROOT_RECORD_ID_HEX: &str = "52e987f294c7ca8de6f98fb1a9bf16307e6bc7fd6bfa582a885eaded2ef14ae0";

#[test]
fn acl_record_id_is_pinned() {
    use outpost_protocol::TerritoryID;
    use outpost_protocol::acl::{ACLPayload, RecordBody};
    use outpost_protocol::crypto::KemSecretKey;

    let record = RecordBody {
        prev: None,
        author: vector_key().verifying_key(),
        payload: ACLPayload::Root {
            territory_seed: [0xA5; 32],
            owner_kem: KemSecretKey::from_seed([0x0F; 32]).public_key(),
            epoch0_key_wrapped: vec![0xEE; 8],
        },
    }
    .sign(&vector_key())
    .unwrap();

    let id = RecordID::of(&record).unwrap();
    assert_eq!(id.to_string(), ROOT_RECORD_ID_HEX);
    // TerritoryID mirrors the root RecordID bytes as a distinct type.
    assert_eq!(
        TerritoryID::of_root(&record).unwrap().as_bytes(),
        id.as_bytes()
    );
    assert_eq!(record.parse_and_verify().unwrap().prev, None);
}
