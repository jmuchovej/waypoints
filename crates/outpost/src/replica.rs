//! The shared replication engine (design doc §12). A [`Replica`] composes a
//! per-Territory [`TerritoryHold`] with the folded ACL state and the §5.4
//! validation loop, and implements the fabric [`SyncPeer`] seam. Both
//! roles — [`station`](crate::station) and [`vessel`](crate::vessel) — run
//! *this same engine* (§3 rule 5); it lives in its own module so the
//! identical replication code is shared, not duplicated in each role.
//!
//! What a Replica does not do is role-specific: it never originates edits
//! (that is the Vessel's alone), it holds no read keys or decryption
//! (Vessel-only), and it assigns no ACL order (the Station orderer's job).
//! It ingests, validates, stores, and answers sync — the common half.

use std::sync::atomic::{AtomicI64, Ordering};

use outpost_fabric::SyncPeer;
use outpost_hold::{AclIndex, HoldError, NoticeIndex, TerritoryHold};
use outpost_protocol::acl::{ACLRecord, ACLReject};
use outpost_protocol::sync::TallyDigest;
use outpost_protocol::{NoticeID, RecordID, SignedNotice, TrackID, Validated, validate};

#[derive(Debug, thiserror::Error)]
pub enum ReplicaError {
    #[error(transparent)]
    Hold(#[from] HoldError),
    #[error(transparent)]
    Wire(#[from] outpost_protocol::WireError),
    #[error("acl log is empty — the Territory has no root record yet")]
    NoAcl,
    #[error("acl record rejected by the fold: {0}")]
    Acl(#[from] ACLReject),
}

/// A Territory replica: the hold, the folded ACL, and the topo cursor.
/// Cheap-cloneless — one per open Territory per peer.
pub struct Replica {
    hold: TerritoryHold,
    acl: AclIndex,
    acl_head: RecordID,
    next_topo: AtomicI64,
}

impl Replica {
    /// Open a Territory from a bootstrap ACL log (in order). The log must be
    /// non-empty — a Territory is defined by its root ACL record (§9).
    pub async fn bootstrap(path: &str, acl_log: &[ACLRecord]) -> Result<Self, ReplicaError> {
        let hold = TerritoryHold::open(path).await?;
        for (position, record) in acl_log.iter().enumerate() {
            hold.append_acl(record, position as u64).await?;
        }
        Self::reopen(hold).await
    }

    /// Reopen a Replica over an already-populated hold (its ACL log is read
    /// back and refolded).
    pub async fn reopen(hold: TerritoryHold) -> Result<Self, ReplicaError> {
        let acl = hold.acl_index().await?;
        let acl_head = acl.head().ok_or(ReplicaError::NoAcl)?;
        let next_topo = AtomicI64::new(hold.max_topo().await? + 1);
        Ok(Self {
            hold,
            acl,
            acl_head,
            next_topo,
        })
    }

    /// The ACL head every locally-authored Notice pins (§8.3).
    pub fn acl_head(&self) -> RecordID {
        self.acl_head
    }

    /// The folded ACL state provider, for authoring/validation decisions.
    pub fn acl(&self) -> &AclIndex {
        &self.acl
    }

    /// Read access to the underlying hold (heads, track queries, records).
    pub fn hold(&self) -> &TerritoryHold {
        &self.hold
    }

    fn take_topo(&self) -> i64 {
        self.next_topo.fetch_add(1, Ordering::SeqCst)
    }

    /// Apply a batch of received Notices: validate each (§5.4), append the
    /// ready ones, and retry the pending (orphan-queued) ones until no
    /// further progress. Duplicates are skipped, so re-delivery is
    /// idempotent. Rejected Notices are dropped (terminal, non-gossiped).
    pub async fn ingest(&self, incoming: Vec<SignedNotice>) -> Result<(), ReplicaError> {
        let mut queue = incoming;
        loop {
            let store = NoticeIndex::snapshot(&self.hold).await?;
            let mut pending = Vec::new();
            let mut progressed = false;
            for notice in queue {
                let id = NoticeID::of(&notice)?;
                if self.hold.get_notice(id).await?.is_some() {
                    continue; // already held
                }
                match validate(&notice, &self.acl, &store) {
                    Ok(Validated::Ready { envelope, .. }) => {
                        let track = if envelope.header.parents.is_empty() {
                            TrackID::from(id)
                        } else {
                            // Parent is present (Ready ⇒ all parents known);
                            // inherit its Track.
                            match self.hold.get_notice(envelope.header.parents[0]).await? {
                                Some(parent) => parent.track,
                                None => {
                                    pending.push(notice);
                                    continue;
                                }
                            }
                        };
                        self.hold
                            .append_notice(&notice, track, self.take_topo())
                            .await?;
                        progressed = true;
                    }
                    Ok(Validated::Pending { .. }) => pending.push(notice),
                    Err(_) => {} // invalid: drop it
                }
            }
            queue = pending;
            if queue.is_empty() || !progressed {
                break;
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl SyncPeer for Replica {
    async fn digest(&self) -> TallyDigest {
        let heads = self.hold.all_heads().await.unwrap_or_default();
        TallyDigest::of_heads(&heads)
    }

    async fn track_notices(&self, track: TrackID) -> Vec<SignedNotice> {
        self.hold
            .track_notices(track)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.notice)
            .collect()
    }

    async fn held_ids(&self) -> std::collections::BTreeSet<NoticeID> {
        self.hold.held_ids().await.unwrap_or_default()
    }

    async fn ingest(&self, notices: Vec<SignedNotice>) {
        // The SyncPeer contract is fire-and-forget; a storage error here is
        // logged and dropped rather than aborting the session.
        if let Err(e) = Replica::ingest(self, notices).await {
            tracing::warn!("replica ingest failed: {e}");
        }
    }
}
