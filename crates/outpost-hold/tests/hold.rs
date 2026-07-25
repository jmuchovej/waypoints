//! Real-engine tests for the per-Territory hold. Everything runs
//! against a live bedrock database — no SQL-string assertions, which would
//! be circular (they never prove valid turso SQL). The end-to-end append →
//! validate flow proves the persistence layer feeds the pure validator
//! correctly.

use ed25519_dalek::{Signer, SigningKey};
use outpost_hold::{AclIndex, NoticeIndex, TerritoryHold};
use outpost_protocol::acl::{
    ACLPayload, ACLRecord, ACLState, InviteKey, Permissions, RecordBody, join_binding,
};
use outpost_protocol::crypto::KemSecretKey;
use outpost_protocol::{
    Envelope, EnvelopeHeader, EpochID, MemberID, NoticeID, RecordID, SignedNotice, TrackID,
    Validated, validate,
};

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn acl_record(author: &SigningKey, prev: Option<RecordID>, payload: ACLPayload) -> ACLRecord {
    RecordBody {
        prev,
        author: author.verifying_key(),
        payload,
    }
    .sign(author)
    .unwrap()
}

/// Build (and persist into `hold`) an Owner+Writer ACL log; return the
/// folded index and the writer key. Records are appended at their log
/// positions, exactly as a validated append would.
async fn seed_acl(hold: &TerritoryHold) -> (AclIndex, SigningKey) {
    let owner = key(1);
    let writer = key(2);

    let root = acl_record(
        &owner,
        None,
        ACLPayload::Root {
            territory_seed: [0xA5; 32],
            owner_kem: KemSecretKey::from_seed([1; 32]).public_key(),
            epoch0_key_wrapped: vec![0xEE; 8],
        },
    );
    let mut fold = AclIndex::new();
    fold.push(&root).unwrap();
    hold.append_acl(&root, 0, None).await.unwrap();

    let invite_secret = key(9);
    let invite = InviteKey::from_bytes(invite_secret.verifying_key().to_bytes());
    let inv = acl_record(
        &owner,
        fold.head(),
        ACLPayload::Invite {
            invite_key: invite,
            permissions: Permissions::Writer,
        },
    );
    fold.push(&inv).unwrap();
    hold.append_acl(&inv, 1, None).await.unwrap();

    let writer_kem = KemSecretKey::from_seed([2; 32]).public_key();
    let proof = invite_secret.sign(&join_binding(&invite, &writer.verifying_key(), &writer_kem));
    let join = acl_record(
        &writer,
        fold.head(),
        ACLPayload::Join {
            invite_key: invite,
            member_kem: writer_kem,
            invite_proof: proof,
        },
    );
    fold.push(&join).unwrap();
    hold.append_acl(&join, 2, None).await.unwrap();

    let approve = acl_record(
        &owner,
        fold.head(),
        ACLPayload::Approve {
            member: MemberID::from(&writer.verifying_key()),
            permissions: Permissions::Writer,
            current_key_wrapped: vec![0xEE; 8],
        },
    );
    fold.push(&approve).unwrap();
    hold.append_acl(&approve, 3, None).await.unwrap();

    (fold, writer)
}

fn notice(author: &SigningKey, parents: Vec<NoticeID>, acl_head: RecordID) -> SignedNotice {
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
                epoch: EpochID::ZERO,
                author: author.verifying_key(),
                snapshot_base,
                is_snapshot: false,
                payload_type: "waypoints.test.v1".into(),
                timestamp: 1,
            },
            payload: vec![0xCC; 8],
        },
        author,
    )
    .unwrap()
}

#[tokio::test]
async fn acl_log_persists_and_refolds() {
    let hold = TerritoryHold::open(":memory:").await.unwrap();
    let (fold, _) = seed_acl(&hold).await;

    // A fresh index rebuilt purely from storage matches the live fold.
    let reloaded = hold.acl_index().await.unwrap();
    assert_eq!(reloaded.head(), fold.head());
    assert_eq!(reloaded.head_state().unwrap().members.len(), 2);
    assert_eq!(hold.acl_records().await.unwrap().len(), 4);
}

#[tokio::test]
async fn append_notice_tracks_heads() {
    let hold = TerritoryHold::open(":memory:").await.unwrap();
    let (fold, writer) = seed_acl(&hold).await;
    let head = fold.head().unwrap();

    // Root notice of a track (parents empty ⇒ it is the track root).
    let root = notice(&writer, vec![], head);
    let root_id = NoticeID::of(&root).unwrap();
    let track = TrackID::from(root_id);
    hold.append_notice(&root, track, 0).await.unwrap();

    // Heads = {root} so far.
    let heads = hold.track_heads(track).await.unwrap();
    assert_eq!(heads.len(), 1);
    assert_eq!(heads[0].id, root_id);
    assert!(heads[0].is_head);

    // A child descends from root; root stops being a head.
    let child = notice(&writer, vec![root_id], head);
    let child_id = NoticeID::of(&child).unwrap();
    hold.append_notice(&child, track, 1).await.unwrap();

    let heads = hold.track_heads(track).await.unwrap();
    assert_eq!(heads.len(), 1);
    assert_eq!(heads[0].id, child_id);

    // Two concurrent children of the same root ⇒ two heads. Give root2 a
    // distinct timestamp so it is not byte-identical to `root`.
    let mut root2_env = notice(&writer, vec![], head).parse_and_verify().unwrap();
    root2_env.header.timestamp = 555;
    let root2 = SignedNotice::sign(&root2_env, &writer).unwrap();
    let root2_id = NoticeID::of(&root2).unwrap();
    let track2 = TrackID::from(root2_id);
    hold.append_notice(&root2, track2, 2).await.unwrap();

    // Two children of root2, distinguished by timestamp so their ids differ.
    let a = notice(&writer, vec![root2_id], head);
    let mut b_env = a.parse_and_verify().unwrap();
    b_env.header.timestamp = 999;
    let b = SignedNotice::sign(&b_env, &writer).unwrap();
    hold.append_notice(&a, track2, 3).await.unwrap();
    hold.append_notice(&b, track2, 4).await.unwrap();
    assert_eq!(hold.track_heads(track2).await.unwrap().len(), 2);
}

#[tokio::test]
async fn notices_after_is_ordered_and_bounded() {
    let hold = TerritoryHold::open(":memory:").await.unwrap();
    let (fold, writer) = seed_acl(&hold).await;
    let head = fold.head().unwrap();

    assert_eq!(hold.max_topo().await.unwrap(), -1);
    for topo in 0..3 {
        let mut env = notice(&writer, vec![], head).parse_and_verify().unwrap();
        env.header.timestamp = topo;
        let n = SignedNotice::sign(&env, &writer).unwrap();
        let track = TrackID::from(NoticeID::of(&n).unwrap());
        hold.append_notice(&n, track, topo).await.unwrap();
    }

    assert_eq!(hold.max_topo().await.unwrap(), 2);
    let all = hold.notices_after(-1).await.unwrap();
    assert_eq!(all.len(), 3);
    assert!(all.windows(2).all(|w| w[0].topo < w[1].topo));
    assert_eq!(hold.notices_after(0).await.unwrap().len(), 2);
    assert_eq!(hold.notices_after(2).await.unwrap().len(), 0);
}

#[tokio::test]
async fn stored_notices_feed_the_validator() {
    let hold = TerritoryHold::open(":memory:").await.unwrap();
    let (fold, writer) = seed_acl(&hold).await;
    let head = fold.head().unwrap();

    // A root notice validates against the materialized ACL index.
    let root = notice(&writer, vec![], head);
    let root_id = NoticeID::of(&root).unwrap();
    let store = NoticeIndex::snapshot(&hold).await.unwrap();
    assert!(matches!(
        validate(&root, &fold, &store).unwrap(),
        Validated::Ready { .. }
    ));
    hold.append_notice(&root, TrackID::from(root_id), 0)
        .await
        .unwrap();

    // A child whose parent is now stored is Ready; before that it is
    // Pending on the missing parent.
    let ghost = NoticeID::from_bytes([0xAB; 32]);
    let orphan = notice(&writer, vec![ghost], head);
    let store = NoticeIndex::snapshot(&hold).await.unwrap();
    match validate(&orphan, &fold, &store).unwrap() {
        Validated::Pending { missing, .. } => assert_eq!(missing, vec![ghost]),
        other => panic!("expected pending, got {other:?}"),
    }

    let child = notice(&writer, vec![root_id], head);
    let store = NoticeIndex::snapshot(&hold).await.unwrap();
    assert!(matches!(
        validate(&child, &fold, &store).unwrap(),
        Validated::Ready { .. }
    ));
}

#[tokio::test]
async fn orphan_queue_parks_and_releases() {
    let hold = TerritoryHold::open(":memory:").await.unwrap();
    let (fold, writer) = seed_acl(&hold).await;
    let head = fold.head().unwrap();

    let parent = NoticeID::from_bytes([0xAB; 32]);
    let child = notice(&writer, vec![parent], head);
    let child_id = NoticeID::of(&child).unwrap();

    hold.park_pending(&child, parent).await.unwrap();
    let waiting = hold.pending_on(parent).await.unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(NoticeID::of(&waiting[0]).unwrap(), child_id);

    hold.unpark(child_id).await.unwrap();
    assert!(hold.pending_on(parent).await.unwrap().is_empty());
}

#[tokio::test]
async fn per_territory_databases_are_isolated() {
    let dir = tempfile::tempdir().unwrap();
    let path_a = dir.path().join("a.db");
    let path_b = dir.path().join("b.db");

    let hold_a = TerritoryHold::open(path_a.to_str().unwrap()).await.unwrap();
    let hold_b = TerritoryHold::open(path_b.to_str().unwrap()).await.unwrap();
    let (fold, writer) = seed_acl(&hold_a).await;
    let root = notice(&writer, vec![], fold.head().unwrap());
    let id = NoticeID::of(&root).unwrap();
    hold_a
        .append_notice(&root, TrackID::from(id), 0)
        .await
        .unwrap();

    // Territory B shares nothing with A.
    assert!(hold_b.get_notice(id).await.unwrap().is_none());
    assert_eq!(hold_b.acl_records().await.unwrap().len(), 0);
    assert!(hold_a.get_notice(id).await.unwrap().is_some());

    // Reopening A sees its persisted state (real file, not :memory:).
    drop(hold_a);
    let reopened = TerritoryHold::open(path_a.to_str().unwrap()).await.unwrap();
    assert!(reopened.get_notice(id).await.unwrap().is_some());
    assert_eq!(reopened.acl_records().await.unwrap().len(), 4);
}
