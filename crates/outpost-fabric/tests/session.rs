//! Session convergence over a framed stream, with an in-memory
//! [`SyncPeer`] stub. Proves the §10 exchange drives two peers to
//! agreement independently of any transport — run over a split tokio
//! duplex pipe, no network. The iroh loopback test (`loopback.rs`) runs the
//! same flow over real QUIC.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use async_trait::async_trait;
use ed25519_dalek::{Signer, SigningKey};
use outpost_fabric::{SyncPeer, initiate, serve};
use outpost_hold::{AclIndex, NoticeIndex, TerritoryHold};
use outpost_protocol::acl::{
    ACLPayload, ACLRecord, InviteKey, Permissions, RecordBody, join_binding,
};
use outpost_protocol::crypto::KemSecretKey;
use outpost_protocol::sync::TallyDigest;
use outpost_protocol::{
    Envelope, EnvelopeHeader, EpochID, MemberID, NoticeID, RecordID, SignedNotice, TrackID,
};
use tokio::io::split;

// A hold-backed SyncPeer — the same shape the role crate will ship in
// slice 7, here as a test double so fabric depends on hold only in tests.
struct HoldPeer {
    hold: TerritoryHold,
    acl: AclIndex,
    acl_head: RecordID,
    next_topo: Mutex<i64>,
}

impl HoldPeer {
    async fn new(records: &[ACLRecord]) -> Self {
        let hold = TerritoryHold::open(":memory:").await.unwrap();
        for (position, record) in records.iter().enumerate() {
            hold.append_acl(record, position as u64).await.unwrap();
        }
        let acl = hold.acl_index().await.unwrap();
        let acl_head = acl.head().unwrap();
        let next_topo = Mutex::new(hold.max_topo().await.unwrap() + 1);
        Self {
            hold,
            acl,
            acl_head,
            next_topo,
        }
    }

    fn take_topo(&self) -> i64 {
        let mut t = self.next_topo.lock().unwrap();
        let v = *t;
        *t += 1;
        v
    }

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
                    parents,
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
}

#[async_trait]
impl SyncPeer for HoldPeer {
    async fn digest(&self) -> TallyDigest {
        TallyDigest::of_heads(&self.hold.all_heads().await.unwrap())
    }

    async fn track_notices(&self, track: TrackID) -> Vec<SignedNotice> {
        self.hold
            .track_notices(track)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.notice)
            .collect()
    }

    async fn held_ids(&self) -> BTreeSet<NoticeID> {
        self.hold.held_ids().await.unwrap()
    }

    async fn ingest(&self, incoming: Vec<SignedNotice>) {
        let mut queue = incoming;
        loop {
            let store = NoticeIndex::snapshot(&self.hold).await.unwrap();
            let mut pending = Vec::new();
            let mut progressed = false;
            for n in queue {
                let id = NoticeID::of(&n).unwrap();
                if self.hold.get_notice(id).await.unwrap().is_some() {
                    continue;
                }
                match outpost_protocol::validate(&n, &self.acl, &store).unwrap() {
                    outpost_protocol::Validated::Ready { envelope, .. } => {
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
                        self.hold
                            .append_notice(&n, track, self.take_topo())
                            .await
                            .unwrap();
                        progressed = true;
                    }
                    outpost_protocol::Validated::Pending { .. } => pending.push(n),
                }
            }
            queue = pending;
            if queue.is_empty() || !progressed {
                break;
            }
        }
    }
}

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn shared_acl() -> (Vec<ACLRecord>, SigningKey) {
    let owner = key(1);
    let writer = key(2);
    let sign = |a: &SigningKey, prev, p| {
        RecordBody {
            prev,
            author: a.verifying_key(),
            payload: p,
        }
        .sign(a)
        .unwrap()
    };
    let mut records = Vec::new();
    let mut fold = AclIndex::new();

    let root = sign(
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
    let inv = sign(
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
    let join = sign(
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

    let approve = sign(
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

#[tokio::test]
async fn session_converges_over_duplex() {
    let (acl, writer) = shared_acl();
    let a = HoldPeer::new(&acl).await;
    let b = HoldPeer::new(&acl).await;

    // A holds a chain, B holds a concurrent edit off the shared root.
    let root = a.author(&writer, vec![], 1).await;
    let root_id = NoticeID::of(&root).unwrap();
    b.ingest(vec![root.clone()]).await;
    let _a_child = a.author(&writer, vec![root_id], 10).await;
    let _b_child = b.author(&writer, vec![root_id], 20).await;

    // Wire the two halves of a duplex pipe to initiator and responder.
    let (a_io, b_io) = tokio::io::duplex(64 * 1024);
    let (mut a_recv, mut a_send) = split(a_io);
    let (mut b_recv, mut b_send) = split(b_io);

    let init = async { initiate(&a, &mut a_send, &mut a_recv).await.unwrap() };
    let resp = async { serve(&b, &mut b_send, &mut b_recv).await.unwrap() };
    tokio::join!(init, resp);

    // Both peers now hold all three Notices; digests agree.
    let want: BTreeSet<NoticeID> = [&root, &_a_child, &_b_child]
        .iter()
        .map(|n| NoticeID::of(n).unwrap())
        .collect();
    assert_eq!(a.held_ids().await, want);
    assert_eq!(b.held_ids().await, want);
    assert!(a.digest().await.divergent(&b.digest().await).is_empty());
    assert_eq!(
        a.hold
            .track_heads(TrackID::from(root_id))
            .await
            .unwrap()
            .len(),
        2
    );
}
