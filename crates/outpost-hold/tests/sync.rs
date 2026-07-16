//! In-process two-peer convergence (§10). Two holds run the *real*
//! protocol code — the pure `outpost_protocol::sync` reconciliation plus
//! `validate` plus the hold's storage — with no network in between. The
//! ingest loop here is the slice-5 driver; the role crates (slice 7) grow
//! the production version of the same orchestration.

use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signer, SigningKey};
use outpost_hold::{AclIndex, NoticeIndex, TerritoryHold};
use outpost_protocol::acl::{
    ACLPayload, ACLRecord, InviteKey, Permissions, RecordBody, join_binding,
};
use outpost_protocol::crypto::KemSecretKey;
use outpost_protocol::sync::{SyncMessage, TallyDigest, wanted};
use outpost_protocol::{
    Envelope, EnvelopeHeader, EpochID, MemberID, NoticeID, RecordID, SignedNotice, TrackID,
    Validated, validate,
};

// ---- fixtures -------------------------------------------------------------

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

/// The shared ACL log both peers start from: Owner (key 1) + Writer (key 2).
fn shared_acl() -> (Vec<ACLRecord>, SigningKey) {
    let owner = key(1);
    let writer = key(2);
    let mut records = Vec::new();
    let mut fold = AclIndex::new();

    let root = acl_record(
        &owner,
        None,
        ACLPayload::Root {
            territory_seed: [0xA5; 32],
            owner_kem: KemSecretKey::from_seed([1; 32]).public_key(),
            epoch0_key_wrapped: vec![0xEE; 8],
        },
    );
    fold.push(&root).unwrap();
    records.push(root);

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
    records.push(inv);

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
    records.push(join);

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
    records.push(approve);

    (records, writer)
}

/// A peer: one hold plus the folded ACL both sides agree on.
struct Peer {
    hold: TerritoryHold,
    acl: AclIndex,
    acl_head: RecordID,
}

impl Peer {
    async fn new(records: &[ACLRecord]) -> Self {
        let hold = TerritoryHold::open(":memory:").await.unwrap();
        for (position, record) in records.iter().enumerate() {
            hold.append_acl(record, position as u64).await.unwrap();
        }
        let acl = hold.acl_index().await.unwrap();
        let acl_head = acl.head().unwrap();
        Self {
            hold,
            acl,
            acl_head,
        }
    }

    /// Author a Notice as `writer`, append it locally, return it.
    async fn author(
        &self,
        writer: &SigningKey,
        parents: Vec<NoticeID>,
        stamp: i64,
    ) -> SignedNotice {
        let snapshot_base = if parents.is_empty() {
            NoticeID::SELF
        } else {
            NoticeID::from_bytes([0x77; 32])
        };
        let n = SignedNotice::sign(
            &Envelope {
                header: EnvelopeHeader {
                    parents: parents.clone(),
                    acl_head: self.acl_head,
                    epoch: EpochID::ZERO,
                    author: writer.verifying_key(),
                    snapshot_base,
                    is_snapshot: false,
                    payload_type: "waypoints.test.v1".into(),
                    timestamp: stamp,
                },
                payload: vec![0xCC; 8],
            },
            writer,
        )
        .unwrap();
        self.ingest(vec![n.clone()]).await;
        n
    }

    /// Apply a batch through validate → append, retrying orphans until no
    /// further progress (the §5.4 pending-queue loop). Duplicates are
    /// skipped, so re-delivery is idempotent.
    async fn ingest(&self, incoming: Vec<SignedNotice>) {
        let mut queue = incoming;
        loop {
            let store = NoticeIndex::snapshot(&self.hold).await.unwrap();
            let mut pending = Vec::new();
            let mut progressed = false;
            for n in queue {
                let id = NoticeID::of(&n).unwrap();
                if self.hold.get_notice(id).await.unwrap().is_some() {
                    continue; // already held
                }
                match validate(&n, &self.acl, &store).unwrap() {
                    Validated::Ready { envelope, .. } => {
                        let track = if envelope.header.parents.is_empty() {
                            TrackID::from(id)
                        } else {
                            self.hold
                                .get_notice(envelope.header.parents[0])
                                .await
                                .unwrap()
                                .unwrap()
                                .track
                        };
                        let topo = self.hold.max_topo().await.unwrap() + 1;
                        self.hold.append_notice(&n, track, topo).await.unwrap();
                        progressed = true;
                    }
                    Validated::Pending { .. } => pending.push(n),
                }
            }
            queue = pending;
            if queue.is_empty() || !progressed {
                break;
            }
        }
    }

    async fn digest(&self) -> TallyDigest {
        TallyDigest::of_heads(&self.hold.all_heads().await.unwrap())
    }
}

/// Run one full initiator→responder sync (§10). Both directions transfer,
/// so a single call reconciles the pair completely.
async fn sync(initiator: &Peer, responder: &Peer) {
    // 1. Initiator sends its digest.
    let request = SyncMessage::TallyRequest(initiator.digest().await);
    let SyncMessage::TallyRequest(init_digest) = &request else {
        unreachable!()
    };

    // 2. Responder finds divergent tracks and offers its ids for each.
    let divergent = responder.digest().await.divergent(init_digest);
    let mut offer: BTreeMap<TrackID, Vec<NoticeID>> = BTreeMap::new();
    for track in &divergent {
        let ids = responder
            .hold
            .track_notices(*track)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        offer.insert(*track, ids);
    }
    let response = SyncMessage::TallyResponse { offer };
    let SyncMessage::TallyResponse { offer } = &response else {
        unreachable!()
    };

    // 3. Initiator computes what to pull and what to push.
    let init_have = initiator.hold.held_ids().await.unwrap();
    let mut want = Vec::new();
    for ids in offer.values() {
        want.extend(wanted(&init_have, ids));
    }
    let offered: BTreeSet<NoticeID> = offer.values().flatten().copied().collect();
    let mut push = Vec::new();
    for track in &divergent {
        for stored in initiator.hold.track_notices(*track).await.unwrap() {
            if !offered.contains(&stored.id) {
                push.push(stored.notice);
            }
        }
    }
    let resupply = SyncMessage::ResupplyRequest { want, push };
    let SyncMessage::ResupplyRequest { want, push } = resupply else {
        unreachable!()
    };

    // 4. Responder applies the pushed notices, returns the wanted ones.
    responder.ingest(push).await;
    let want_set: BTreeSet<NoticeID> = want.into_iter().collect();
    let mut notices = Vec::new();
    for track in &divergent {
        for stored in responder.hold.track_notices(*track).await.unwrap() {
            if want_set.contains(&stored.id) {
                notices.push(stored.notice);
            }
        }
    }
    let reply = SyncMessage::ResupplyResponse { notices };
    let SyncMessage::ResupplyResponse { notices } = reply else {
        unreachable!()
    };

    // 5. Initiator applies the pulled notices.
    initiator.ingest(notices).await;
}

// ---- tests ----------------------------------------------------------------

#[tokio::test]
async fn already_in_sync_finds_no_divergence() {
    let (acl, writer) = shared_acl();
    let a = Peer::new(&acl).await;
    let b = Peer::new(&acl).await;
    let root = a.author(&writer, vec![], 1).await;
    b.ingest(vec![root]).await;

    assert!(a.digest().await.divergent(&b.digest().await).is_empty());
}

#[tokio::test]
async fn one_sided_history_transfers() {
    let (acl, writer) = shared_acl();
    let a = Peer::new(&acl).await;
    let b = Peer::new(&acl).await;

    // A builds a three-Notice chain B has never seen.
    let root = a.author(&writer, vec![], 1).await;
    let root_id = NoticeID::of(&root).unwrap();
    let mid = a.author(&writer, vec![root_id], 2).await;
    let mid_id = NoticeID::of(&mid).unwrap();
    let _tip = a.author(&writer, vec![mid_id], 3).await;

    sync(&a, &b).await;

    // B now holds all three and agrees on heads.
    assert_eq!(b.hold.held_ids().await.unwrap().len(), 3);
    assert!(a.digest().await.divergent(&b.digest().await).is_empty());
}

#[tokio::test]
async fn concurrent_divergent_edits_converge_bidirectionally() {
    let (acl, writer) = shared_acl();
    let a = Peer::new(&acl).await;
    let b = Peer::new(&acl).await;

    // Shared root, then each peer extends it independently (two heads).
    let root = a.author(&writer, vec![], 1).await;
    let root_id = NoticeID::of(&root).unwrap();
    b.ingest(vec![root.clone()]).await;

    let a_child = a.author(&writer, vec![root_id], 10).await;
    let b_child = b.author(&writer, vec![root_id], 20).await;
    assert_ne!(
        NoticeID::of(&a_child).unwrap(),
        NoticeID::of(&b_child).unwrap()
    );

    sync(&a, &b).await;

    // Both peers hold all three Notices and see the same two heads.
    let want: BTreeSet<NoticeID> = [&root, &a_child, &b_child]
        .iter()
        .map(|n| NoticeID::of(n).unwrap())
        .collect();
    assert_eq!(a.hold.held_ids().await.unwrap(), want);
    assert_eq!(b.hold.held_ids().await.unwrap(), want);
    assert!(a.digest().await.divergent(&b.digest().await).is_empty());

    let heads = a.hold.track_heads(TrackID::from(root_id)).await.unwrap();
    assert_eq!(heads.len(), 2);
}

#[tokio::test]
async fn out_of_order_delivery_is_orphan_queued_then_resolved() {
    let (acl, writer) = shared_acl();
    let a = Peer::new(&acl).await;
    let b = Peer::new(&acl).await;

    let root = a.author(&writer, vec![], 1).await;
    let root_id = NoticeID::of(&root).unwrap();
    let mid = a.author(&writer, vec![root_id], 2).await;
    let mid_id = NoticeID::of(&mid).unwrap();
    let tip = a.author(&writer, vec![mid_id], 3).await;

    // Deliver to B in reverse order: tip and mid are orphans until root
    // arrives; the ingest retry loop drains them.
    b.ingest(vec![tip, mid, root]).await;

    assert_eq!(b.hold.held_ids().await.unwrap().len(), 3);
    assert!(a.digest().await.divergent(&b.digest().await).is_empty());
}
