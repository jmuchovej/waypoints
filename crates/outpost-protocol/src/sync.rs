//! The two-tier sync protocol (§10): a **Tally** finds divergence, a
//! **Resupply** closes it. This module is the *pure* half — message types
//! and the reconciliation computations over head sets and id sets. The
//! I/O half (open a session, drive these over a transport, apply received
//! Notices through [`validate`](crate::validate)) belongs to the fabric
//! and role crates; nothing here does storage or networking.
//!
//! Scope for this slice: correct set reconciliation over `(Track → heads)`.
//! The design doc's logarithmic *range-diff* refinement (recursing into
//! mismatched ID ranges) is a later optimization — the digest here compares
//! whole tracks, which is correct and simple; the message shape already
//! carries per-track granularity so the recursion can slot in without a
//! wire change. Snapshot-path negotiation (§6.3) is likewise deferred with
//! the snapshot machinery.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::id::{NoticeID, TrackID};

/// A compact summary of a peer's state: per Track, a hash of that Track's
/// head set (§10.1). Two peers in sync produce byte-identical digests;
/// any mismatch localizes to the differing Tracks.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TallyDigest {
    /// Track → BLAKE3 over its sorted head ids. A Track absent here has no
    /// Notices on that peer.
    entries: BTreeMap<TrackID, [u8; 32]>,
}

impl TallyDigest {
    /// Summarize a peer's `(Track → heads)` map. Heads are hashed in
    /// sorted order so the digest is independent of enumeration order.
    pub fn of_heads(heads: &BTreeMap<TrackID, BTreeSet<NoticeID>>) -> Self {
        let entries = heads
            .iter()
            .map(|(track, ids)| (*track, hash_ids(ids)))
            .collect();
        Self { entries }
    }

    /// The Tracks whose head sets differ between two peers: present on one
    /// side only, or present on both with different heads (§10.1). This is
    /// symmetric — both peers compute the same divergent set.
    pub fn divergent(&self, other: &TallyDigest) -> BTreeSet<TrackID> {
        let mut tracks = BTreeSet::new();
        for (track, digest) in &self.entries {
            if other.entries.get(track) != Some(digest) {
                tracks.insert(*track);
            }
        }
        for track in other.entries.keys() {
            if !self.entries.contains_key(track) {
                tracks.insert(*track);
            }
        }
        tracks
    }
}

fn hash_ids(ids: &BTreeSet<NoticeID>) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for id in ids {
        hasher.update(id.as_bytes());
    }
    *hasher.finalize().as_bytes()
}

/// Of the ids a peer offered, the ones we do not already hold — the
/// resupply "give me what I'm missing" set (§10.2). Order-preserving on
/// the offer so the requester controls transfer order.
pub fn wanted<'a>(
    have: &BTreeSet<NoticeID>,
    offered: impl IntoIterator<Item = &'a NoticeID>,
) -> Vec<NoticeID> {
    offered
        .into_iter()
        .filter(|id| !have.contains(id))
        .copied()
        .collect()
}

/// One observed device of one identity, as a Station saw it (§4): the
/// device key that bound a connection, the certificate tying it to the
/// identity that authored through it, and when. This is *attribution*, not
/// authorization — a user reads it to answer "which devices have touched my
/// Territory"; nothing grants or revokes through it. The viewer verifies
/// `cert` against `identity` (and derives the `MemberID` from `identity`)
/// rather than trusting the reporting Station.
///
/// Timestamps are the Station's clock (seconds since the Unix epoch) —
/// presence metadata a Station already observes, same trust class as the
/// envelope. `connected` is live-session presence: `true` only while a
/// session from that device is open.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DeviceSighting {
    /// The identity behind the device (its `MemberID` is derived from this).
    pub identity: ed25519_dalek::VerifyingKey,
    /// The identity's certificate over the device key — verify it. The
    /// certified key (`cert.device`) *is* the sighted device (the
    /// `VesselID`); a Station only records a sighting after checking it was
    /// the connection's authenticated key.
    pub cert: crate::auth::DeviceCert,
    pub first_seen: u64,
    pub last_seen: u64,
    pub connected: bool,
}

impl DeviceSighting {
    /// The sighted device key (the `VesselID`) — the key `cert` certifies.
    pub fn device(&self) -> &ed25519_dalek::VerifyingKey {
        &self.cert.device
    }
}

/// One message in a sync session (§10). A session first reconciles the ACL
/// log (which is linear and must be current before Notices can validate, as
/// each Notice pins an `acl_head`), then the Notice DAG:
///
/// **ACL phase** (§8, linear log):
/// 1. Initiator → [`AclRequest`](SyncMessage::AclRequest) with its log length.
/// 2. Responder → [`AclResponse`](SyncMessage::AclResponse): the records the
///    initiator is missing (its suffix), plus the responder's own length.
/// 3. Initiator → [`AclPush`](SyncMessage::AclPush): the records the responder
///    is missing (empty when the initiator is not ahead).
///
/// **Notice phase** (§10, DAG):
/// 4. Initiator → [`TallyRequest`](SyncMessage::TallyRequest) with its digest.
/// 5. Responder → [`TallyResponse`](SyncMessage::TallyResponse): per divergent
///    Track, the ids it holds (its resupply *offer*).
/// 6. Initiator → [`ResupplyRequest`](SyncMessage::ResupplyRequest): ids to
///    pull, plus Notices to push (bidirectional).
/// 7. Responder → [`ResupplyResponse`](SyncMessage::ResupplyResponse): the
///    pulled Notices; it has already applied the pushed ones.
///
/// Live updates ride the same push shapes over an open session — a receiver
/// treats them identically to reconciled records (§10.2).
// A protocol frame sent one at a time; the size spread between the small
// control variants and the record-carrying ones is inherent, and boxing the
// wire types would only obscure framing for no real gain.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SyncMessage {
    /// The leading frame a Vessel sends on connect (§10.2): its identity and
    /// the certificate proving the connection's device key belongs to that
    /// identity. The Station verifies the cert and that its device key is the
    /// connection's authenticated key.
    Hello {
        identity: ed25519_dalek::VerifyingKey,
        cert: crate::auth::DeviceCert,
    },
    /// Vessel → Station: register a Territory so the Station will hold and
    /// serve it (§13, the coordinator role absorbed into Stations). Carries
    /// the Territory's root ACL record so a Station that does not yet hold it
    /// can bootstrap a fresh hold from it (`TerritoryID` is the root's hash,
    /// so the Station verifies the two agree before serving).
    Register {
        territory: crate::TerritoryID,
        root: crate::acl::ACLRecord,
    },
    /// Station → Vessel: acknowledge a registration (or handshake) — `true`
    /// if accepted.
    Ack {
        ok: bool,
    },
    /// Vessel → Station: scope this session to `territory` before the sync
    /// flow (§9 — sessions are keyed by `(FleetID, TerritoryID)`). The Station
    /// acks `true` if it serves that Territory and then runs the sync flow;
    /// `false` if it does not (the Vessel must `Register` first).
    Open {
        territory: crate::TerritoryID,
    },
    /// Vessel → orderer Station: order this record for `territory` (§8.4). The
    /// Territory is named explicitly because a non-root record's log cannot be
    /// cheaply recovered from the record alone.
    OrderRequest {
        territory: crate::TerritoryID,
        record: crate::acl::ACLRecord,
    },
    /// Orderer → vessel: the countersigned proof, or `None` if the orderer
    /// refused (e.g. the record does not chain from the current head).
    OrderResponse {
        proof: Option<crate::acl::OrderProof>,
    },
    AclRequest {
        /// The initiator's ACL log length (its position in the linear log).
        len: u64,
    },
    AclResponse {
        /// Ordered ACL records the initiator lacks (the responder's suffix),
        /// each carrying its order proof (§8.4).
        records: Vec<crate::acl::OrderedRecord>,
        /// The responder's own log length, so the initiator knows whether to
        /// push its own suffix.
        len: u64,
    },
    AclPush {
        /// Ordered ACL records the responder lacked (empty if none).
        records: Vec<crate::acl::OrderedRecord>,
    },
    /// Vessel → Station: list the device sightings for `territory` (§4 —
    /// which devices, behind which identities, have touched it). Requires a
    /// preceding [`Hello`](SyncMessage::Hello); the Station answers only if
    /// the authenticated identity is a **member** of the Territory (viewing
    /// is gated; being sighted is not).
    DevicesRequest {
        territory: crate::TerritoryID,
    },
    /// Station → Vessel: the sightings. Each carries its device cert so the
    /// *viewer* verifies attribution — the Station's word is never trusted
    /// for a security judgment.
    DevicesResponse {
        devices: Vec<DeviceSighting>,
    },
    TallyRequest(TallyDigest),
    TallyResponse {
        /// Divergent Track → the ids the responder holds for it.
        offer: BTreeMap<TrackID, Vec<NoticeID>>,
    },
    ResupplyRequest {
        /// Ids the initiator wants pulled from the responder.
        want: Vec<NoticeID>,
        /// Notices the initiator is pushing to the responder.
        push: Vec<crate::SignedNotice>,
    },
    ResupplyResponse {
        /// The Notices the initiator requested.
        notices: Vec<crate::SignedNotice>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(seed: u8) -> NoticeID {
        NoticeID::from_bytes([seed; 32])
    }

    fn track(seed: u8) -> TrackID {
        TrackID::from(nid(seed))
    }

    #[test]
    fn identical_states_have_no_divergence() {
        let mut heads = BTreeMap::new();
        heads.insert(track(1), BTreeSet::from([nid(10), nid(11)]));
        let a = TallyDigest::of_heads(&heads);
        let b = TallyDigest::of_heads(&heads);
        assert!(a.divergent(&b).is_empty());
    }

    #[test]
    fn head_digest_is_order_independent() {
        let a = TallyDigest::of_heads(&BTreeMap::from([(
            track(1),
            BTreeSet::from([nid(10), nid(11)]),
        )]));
        // Same set, inserted the other way — BTreeSet normalizes order.
        let b = TallyDigest::of_heads(&BTreeMap::from([(
            track(1),
            BTreeSet::from([nid(11), nid(10)]),
        )]));
        assert!(a.divergent(&b).is_empty());
    }

    #[test]
    fn divergence_localizes_to_changed_tracks() {
        let a = TallyDigest::of_heads(&BTreeMap::from([
            (track(1), BTreeSet::from([nid(10)])),
            (track(2), BTreeSet::from([nid(20)])),
        ]));
        let b = TallyDigest::of_heads(&BTreeMap::from([
            (track(1), BTreeSet::from([nid(10)])),          // same
            (track(2), BTreeSet::from([nid(20), nid(21)])), // grew
            (track(3), BTreeSet::from([nid(30)])),          // only on b
        ]));
        assert_eq!(a.divergent(&b), BTreeSet::from([track(2), track(3)]));
        // Symmetric.
        assert_eq!(b.divergent(&a), BTreeSet::from([track(2), track(3)]));
    }

    #[test]
    fn wanted_filters_to_the_missing() {
        let have = BTreeSet::from([nid(1), nid(2)]);
        let offered = [nid(1), nid(2), nid(3), nid(4)];
        assert_eq!(wanted(&have, &offered), vec![nid(3), nid(4)]);
    }
}
