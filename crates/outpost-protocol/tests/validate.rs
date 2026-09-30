//! Adversarial suite for the validation function. The scenarios a
//! removed member (or a lying peer) would actually try, each pinned as a
//! promised verdict.

use std::collections::BTreeMap;

use ed25519_dalek::{Signer, SigningKey};
use outpost_protocol::KEMSecret;
use outpost_protocol::{
    ACLPayload, ACLRecord, ACLReject, ACLState, InviteKey, Permissions, RecordBody, join_binding,
};
use outpost_protocol::{
    ACLStateProvider, Envelope, EnvelopeHeader, EpochID, MemberID, NoticeID, NoticeStore, RecordID,
    RejectReason, SignedNotice, Validated, validate,
};

/// An in-memory fold of an ordered ACL log: record → (position, state).
/// Runs the real fold — a fixture, not a re-implementation.
struct LogFixture {
    states: BTreeMap<RecordID, (u64, ACLState)>,
    head: RecordID,
    head_state: ACLState,
}

impl LogFixture {
    fn new(root: &ACLRecord) -> Self {
        let state = ACLState::genesis(root).unwrap();
        let head = state.head;
        Self {
            states: BTreeMap::from([(head, (0, state.clone()))]),
            head,
            head_state: state,
        }
    }

    fn push(&mut self, record: &ACLRecord) {
        let next = self.head_state.apply(record).unwrap();
        let position = self.states.len() as u64;
        self.head = next.head;
        self.states.insert(next.head, (position, next.clone()));
        self.head_state = next;
    }
}

impl ACLStateProvider for LogFixture {
    fn state_at(&self, record: RecordID) -> Option<&ACLState> {
        self.states.get(&record).map(|(_, state)| state)
    }

    fn position_of(&self, record: RecordID) -> Option<u64> {
        self.states.get(&record).map(|(position, _)| *position)
    }
}

/// Accepted notices: id → pinned acl_head.
#[derive(Default)]
struct StoreFixture(BTreeMap<NoticeID, RecordID>);

impl NoticeStore for StoreFixture {
    fn contains(&self, notice: NoticeID) -> bool {
        self.0.contains_key(&notice)
    }

    fn acl_head_of(&self, notice: NoticeID) -> Option<RecordID> {
        self.0.get(&notice).copied()
    }
}

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn record(author: &SigningKey, prev: Option<RecordID>, payload: ACLPayload) -> ACLRecord {
    RecordBody {
        prev,
        author: author.verifying_key(),
        payload,
    }
    .sign(author)
    .unwrap()
}

/// Territory with an Owner (key 1) and an approved Writer (key 2), then
/// the Writer removed at the final record. Returns the fixture plus the
/// two interesting heads: `pre_removal` (Writer still a member) and
/// `post_removal`.
fn removed_writer_log() -> (LogFixture, SigningKey, SigningKey, RecordID, RecordID) {
    let owner = key(1);
    let writer = key(2);

    let root = record(
        &owner,
        None,
        ACLPayload::Root {
            territory_seed: [0xA5; 32],
            owner_kem: KEMSecret::from_seed([1; 32]).public_key(),
            epoch0_key_wrapped: vec![0xEE; 8],
        },
    );
    let mut log = LogFixture::new(&root);

    let invite_secret = key(9);
    let invite = InviteKey::from_bytes(invite_secret.verifying_key().to_bytes());
    log.push(&record(
        &owner,
        Some(log.head),
        ACLPayload::Invite {
            invite_key: invite,
            permissions: Permissions::Writer,
        },
    ));

    let writer_kem = KEMSecret::from_seed([2; 32]).public_key();
    let proof = invite_secret.sign(&join_binding(&invite, &writer.verifying_key(), &writer_kem));
    log.push(&record(
        &writer,
        Some(log.head),
        ACLPayload::Join {
            invite_key: invite,
            member_kem: writer_kem,
            invite_proof: proof,
        },
    ));

    log.push(&record(
        &owner,
        Some(log.head),
        ACLPayload::Approve {
            member: MemberID::from(&writer.verifying_key()),
            permissions: Permissions::Writer,
            current_key_wrapped: vec![0xEE; 8],
        },
    ));
    let pre_removal = log.head;

    log.push(&record(
        &owner,
        Some(log.head),
        ACLPayload::Remove {
            member: MemberID::from(&writer.verifying_key()),
            new_epoch: EpochID::ZERO.next(),
            new_key_wrapped: vec![(MemberID::from(&owner.verifying_key()), vec![0xEE; 8])],
            prev_key_wrapped: vec![0xEE; 8],
        },
    ));
    let post_removal = log.head;

    (log, owner, writer, pre_removal, post_removal)
}

fn notice(
    author: &SigningKey,
    parents: Vec<NoticeID>,
    acl_head: RecordID,
    epoch: EpochID,
) -> SignedNotice {
    notice_at(author, parents, acl_head, epoch, 1)
}

fn notice_at(
    author: &SigningKey,
    parents: Vec<NoticeID>,
    acl_head: RecordID,
    epoch: EpochID,
    timestamp: i64,
) -> SignedNotice {
    let snapshot_base = if parents.is_empty() {
        NoticeID::SELF
    } else {
        NoticeID::from_bytes([0x77; 32])
    };
    SignedNotice::sign(
        &Envelope {
            header: EnvelopeHeader {
                parents,
                acl_head,
                epoch,
                author: author.verifying_key(),
                snapshot_base,
                is_snapshot: false,
                payload_type: "waypoints.test.v1".into(),
                timestamp,
            },
            payload: vec![0xCC; 8],
        },
        author,
    )
    .unwrap()
}

#[test]
fn valid_root_notice_is_ready() {
    let (log, _, writer, pre_removal, _) = removed_writer_log();
    let n = notice(&writer, vec![], pre_removal, EpochID::ZERO);
    let verdict = validate(&n, &log, &StoreFixture::default()).unwrap();
    assert!(matches!(verdict, Validated::Ready { id, .. } if id == NoticeID::of(&n).unwrap()));
}

#[test]
fn missing_parents_are_pending_not_rejected() {
    let (log, _, writer, pre_removal, _) = removed_writer_log();
    let ghost = NoticeID::from_bytes([0xAB; 32]);
    let n = notice(&writer, vec![ghost], pre_removal, EpochID::ZERO);
    let verdict = validate(&n, &log, &StoreFixture::default()).unwrap();
    assert!(matches!(
        verdict,
        Validated::Pending { missing, .. } if missing == vec![ghost]
    ));
}

#[test]
fn forged_signature_is_malformed() {
    let (log, _, writer, pre_removal, _) = removed_writer_log();
    let n = notice(&writer, vec![], pre_removal, EpochID::ZERO);
    let forged = SignedNotice {
        bytes: n.bytes,
        signature: key(8).sign(b"unrelated"),
    };
    assert!(matches!(
        validate(&forged, &log, &StoreFixture::default()),
        Err(RejectReason::Malformed(_))
    ));
}

#[test]
fn unknown_acl_head_is_rejected() {
    let (log, _, writer, _, _) = removed_writer_log();
    let n = notice(
        &writer,
        vec![],
        RecordID::from_bytes([0xFE; 32]),
        EpochID::ZERO,
    );
    assert!(matches!(
        validate(&n, &log, &StoreFixture::default()),
        Err(RejectReason::UnknownACLHead)
    ));
}

#[test]
fn non_members_and_readers_cannot_write() {
    let (mut log, owner, _, _, _) = removed_writer_log();
    let outsider = key(7);
    let n = notice(&outsider, vec![], log.head, EpochID::ZERO.next());
    assert!(matches!(
        validate(&n, &log, &StoreFixture::default()),
        Err(RejectReason::Forbidden(ACLReject::NotAMember))
    ));

    // Admit a Reader; readers hold keys but may not write.
    let invite_secret = key(10);
    let invite = InviteKey::from_bytes(invite_secret.verifying_key().to_bytes());
    log.push(&record(
        &owner,
        Some(log.head),
        ACLPayload::Invite {
            invite_key: invite,
            permissions: Permissions::Reader,
        },
    ));
    let reader = key(3);
    let reader_kem = KEMSecret::from_seed([3; 32]).public_key();
    let proof = invite_secret.sign(&join_binding(&invite, &reader.verifying_key(), &reader_kem));
    log.push(&record(
        &reader,
        Some(log.head),
        ACLPayload::Join {
            invite_key: invite,
            member_kem: reader_kem,
            invite_proof: proof,
        },
    ));
    log.push(&record(
        &owner,
        Some(log.head),
        ACLPayload::Approve {
            member: MemberID::from(&reader.verifying_key()),
            permissions: Permissions::Reader,
            current_key_wrapped: vec![0xEE; 8],
        },
    ));

    let n = notice(&reader, vec![], log.head, EpochID::ZERO.next());
    assert!(matches!(
        validate(&n, &log, &StoreFixture::default()),
        Err(RejectReason::Forbidden(ACLReject::InsufficientAuthority))
    ));
}

#[test]
fn removed_member_cannot_pin_post_removal_state() {
    let (log, _, writer, _, post_removal) = removed_writer_log();
    let n = notice(&writer, vec![], post_removal, EpochID::ZERO.next());
    assert!(matches!(
        validate(&n, &log, &StoreFixture::default()),
        Err(RejectReason::Forbidden(ACLReject::NotAMember))
    ));
}

#[test]
fn removed_member_cannot_extend_post_removal_history_with_old_pin() {
    // The backdating attack: pin the pre-removal record (where the
    // author could write) but descend from a Notice pinned post-removal.
    let (log, owner, writer, pre_removal, post_removal) = removed_writer_log();

    let fresh = notice(&owner, vec![], post_removal, EpochID::ZERO.next());
    let fresh_id = NoticeID::of(&fresh).unwrap();
    let mut store = StoreFixture::default();
    store.0.insert(fresh_id, post_removal);

    let backdated = notice(&writer, vec![fresh_id], pre_removal, EpochID::ZERO);
    assert!(matches!(
        validate(&backdated, &log, &store),
        Err(RejectReason::ACLRegression)
    ));
}

#[test]
fn removed_member_extending_pre_removal_history_still_validates() {
    // The honest nuance: extending *pre-removal* heads with a
    // pre-removal pin is structurally indistinguishable from a write made
    // before the removal, and validation alone accepts it. The backstop
    // (ingest quarantine) is service-level by design.
    let (log, _, writer, pre_removal, _) = removed_writer_log();

    let old = notice(&writer, vec![], pre_removal, EpochID::ZERO);
    let old_id = NoticeID::of(&old).unwrap();
    let mut store = StoreFixture::default();
    store.0.insert(old_id, pre_removal);

    let extension = notice(&writer, vec![old_id], pre_removal, EpochID::ZERO);
    assert!(matches!(
        validate(&extension, &log, &store),
        Ok(Validated::Ready { .. })
    ));
}

#[test]
fn epoch_ahead_of_pinned_state_is_rejected() {
    let (log, _, writer, pre_removal, _) = removed_writer_log();
    // Epoch 1 exists at the log head, but the pinned pre-removal state
    // never reached it.
    let n = notice(&writer, vec![], pre_removal, EpochID::ZERO.next());
    assert!(matches!(
        validate(&n, &log, &StoreFixture::default()),
        Err(RejectReason::EpochAhead)
    ));
}

#[test]
fn validation_ignores_the_header_clock() {
    // The clock is advisory at the protocol layer: any value — zero, far
    // past, far future, even negative — leaves the verdict unchanged, for
    // an accepted Notice and a rejected one alike.
    let (log, _, writer, pre_removal, _) = removed_writer_log();
    let clocks = [0, 1, -1, 1_700_000_000_000, i64::MIN, i64::MAX];

    for timestamp in clocks {
        let good = notice_at(&writer, vec![], pre_removal, EpochID::ZERO, timestamp);
        assert!(
            matches!(
                validate(&good, &log, &StoreFixture::default()),
                Ok(Validated::Ready { .. })
            ),
            "clock {timestamp} changed an accept verdict"
        );

        let bad = notice_at(
            &writer,
            vec![],
            RecordID::from_bytes([0xFE; 32]),
            EpochID::ZERO,
            timestamp,
        );
        assert!(
            matches!(
                validate(&bad, &log, &StoreFixture::default()),
                Err(RejectReason::UnknownACLHead)
            ),
            "clock {timestamp} changed a reject verdict"
        );
    }
}

#[test]
fn mutating_the_header_clock_breaks_signature_and_decryption() {
    use outpost_protocol::{ReadKey, open_payload, seal_payload};
    use rand::SeedableRng;

    let (_, _, writer, pre_removal, _) = removed_writer_log();
    let key = ReadKey::from_bytes([0x42; 32]);
    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);

    let header = EnvelopeHeader {
        parents: vec![],
        acl_head: pre_removal,
        epoch: EpochID::ZERO,
        author: writer.verifying_key(),
        snapshot_base: NoticeID::SELF,
        is_snapshot: false,
        payload_type: "waypoints.test.v1".into(),
        timestamp: 1_000,
    };
    let payload = seal_payload(&mut rng, &key, &header, b"plaintext").unwrap();
    let notice = SignedNotice::sign(
        &Envelope {
            header: header.clone(),
            payload: payload.clone(),
        },
        &writer,
    )
    .unwrap();

    // Control: untampered, the signature verifies and the payload opens.
    let envelope = notice.parse_and_verify().unwrap();
    assert_eq!(
        open_payload(&key, &envelope.header, &envelope.payload).unwrap(),
        b"plaintext"
    );

    // Bump the clock in transit, keeping the original signature.
    let mut tampered_header = header.clone();
    tampered_header.timestamp += 1;
    let tampered = SignedNotice {
        bytes: Envelope {
            header: tampered_header.clone(),
            payload: payload.clone(),
        }
        .encode()
        .unwrap(),
        signature: notice.signature,
    };
    assert!(
        tampered.parse_and_verify().is_err(),
        "signature must not survive a clock mutation"
    );
    assert!(
        open_payload(&key, &tampered_header, &payload).is_err(),
        "AAD binding must not survive a clock mutation"
    );
}

#[test]
fn snapshot_base_sentinel_is_enforced_both_ways() {
    let (log, owner, _, _, post_removal) = removed_writer_log();

    // A root whose snapshot_base is not the sentinel.
    let bad_root = SignedNotice::sign(
        &Envelope {
            header: EnvelopeHeader {
                parents: vec![],
                acl_head: post_removal,
                epoch: EpochID::ZERO,
                author: owner.verifying_key(),
                snapshot_base: NoticeID::from_bytes([0x77; 32]),
                is_snapshot: false,
                payload_type: "waypoints.test.v1".into(),
                timestamp: 1,
            },
            payload: vec![],
        },
        &owner,
    )
    .unwrap();
    assert!(matches!(
        validate(&bad_root, &log, &StoreFixture::default()),
        Err(RejectReason::BadSnapshotBase)
    ));

    // A non-root claiming the sentinel.
    let parent = NoticeID::from_bytes([0x66; 32]);
    let mut store = StoreFixture::default();
    store.0.insert(parent, post_removal);
    let bad_child = SignedNotice::sign(
        &Envelope {
            header: EnvelopeHeader {
                parents: vec![parent],
                acl_head: post_removal,
                epoch: EpochID::ZERO,
                author: owner.verifying_key(),
                snapshot_base: NoticeID::SELF,
                is_snapshot: false,
                payload_type: "waypoints.test.v1".into(),
                timestamp: 1,
            },
            payload: vec![],
        },
        &owner,
    )
    .unwrap();
    assert!(matches!(
        validate(&bad_child, &log, &store),
        Err(RejectReason::BadSnapshotBase)
    ));
}
