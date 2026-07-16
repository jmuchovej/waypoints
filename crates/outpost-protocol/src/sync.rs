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

/// One message in a sync session (§10). The flow is two round-trips:
///
/// 1. Initiator → [`TallyRequest`](SyncMessage::TallyRequest) with its digest.
/// 2. Responder → [`TallyResponse`](SyncMessage::TallyResponse): for each
///    divergent Track, the Notice ids it holds (its resupply *offer*).
/// 3. Initiator → [`ResupplyRequest`](SyncMessage::ResupplyRequest): the
///    ids it wants pulled, plus the Notices it will push (bidirectional).
/// 4. Responder → [`ResupplyResponse`](SyncMessage::ResupplyResponse) with
///    the pulled Notices; it has already applied the pushed ones.
///
/// Live updates ride the same `Push` shape over an open session — a
/// receiver treats them identically to pulled Notices (§10.2).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SyncMessage {
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
