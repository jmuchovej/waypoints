//! The shared replication engine (design doc §12). A [`Replica`] composes a
//! per-Territory [`TerritoryHold`] with the §5.4 validation loop, and
//! implements the fabric [`SyncPeer`] seam. Both roles —
//! [`station`](crate::station) and [`vessel`](crate::vessel) — run *this
//! same engine* (§3 rule 5); it lives in its own module so the identical
//! replication code is shared, not duplicated in each role.
//!
//! What a Replica does not do is role-specific: it never originates edits
//! (that is the Vessel's alone), it holds no read keys or decryption
//! (Vessel-only), and it assigns no ACL order (the Station orderer's job).
//! It ingests, validates, stores, and answers sync — the common half.
//!
//! The ACL state is *derived from the hold* on demand rather than cached, so
//! newly-synced ACL records take effect immediately with no interior
//! mutability. The ACL log is short (membership changes are rare), so
//! refolding it per ingest is cheap.

use std::collections::HashSet;
use std::sync::atomic::{AtomicI64, Ordering};

use ed25519_dalek::VerifyingKey;
use outpost_fabric::SyncPeer;
use outpost_hold::{AclIndex, HoldError, NoticeIndex, TerritoryHold};
use outpost_protocol::acl::{ACLRecord, ACLReject, OrderProof, OrderedRecord};
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

/// A Territory replica: the hold and the topo cursor. ACL and head state are
/// derived from the hold as needed.
pub struct Replica {
    hold: TerritoryHold,
    next_topo: AtomicI64,
    /// The fleet's verifying key, when this Territory is homed to a fleet
    /// (§8.4). When set, ACL records arriving over sync must carry a valid
    /// orderer proof; when `None` (self-hosted solo, or tests), ordering is
    /// not verified.
    fleet_key: Option<VerifyingKey>,
}

impl Replica {
    /// Open a Territory from a bootstrap ACL log (in order), unfleeted —
    /// order proofs are not verified. The log must be non-empty; a Territory
    /// is defined by its root ACL record (§9).
    pub async fn bootstrap(path: &str, acl_log: &[ACLRecord]) -> Result<Self, ReplicaError> {
        Self::bootstrap_inner(path, acl_log, None).await
    }

    /// [`bootstrap`](Self::bootstrap) homed to a fleet: ACL records received
    /// over sync are verified against `fleet_key` (§8.4). The bootstrap log
    /// itself is trusted seed and is not re-verified — home a fleeted
    /// Territory with just its Root and let ordered records arrive by sync.
    pub async fn bootstrap_with_fleet(
        path: &str,
        acl_log: &[ACLRecord],
        fleet_key: VerifyingKey,
    ) -> Result<Self, ReplicaError> {
        Self::bootstrap_inner(path, acl_log, Some(fleet_key)).await
    }

    async fn bootstrap_inner(
        path: &str,
        acl_log: &[ACLRecord],
        fleet_key: Option<VerifyingKey>,
    ) -> Result<Self, ReplicaError> {
        let hold = TerritoryHold::open(path).await?;
        for (position, record) in acl_log.iter().enumerate() {
            hold.append_acl(record, position as u64, None).await?;
        }
        Self::reopen_inner(hold, fleet_key).await
    }

    /// Reopen a Replica over an already-populated hold (unfleeted).
    pub async fn reopen(hold: TerritoryHold) -> Result<Self, ReplicaError> {
        Self::reopen_inner(hold, None).await
    }

    async fn reopen_inner(
        hold: TerritoryHold,
        fleet_key: Option<VerifyingKey>,
    ) -> Result<Self, ReplicaError> {
        // A Territory must have a root; fail fast if the hold is empty.
        if hold.acl_index().await?.head().is_none() {
            return Err(ReplicaError::NoAcl);
        }
        let next_topo = AtomicI64::new(hold.max_topo().await? + 1);
        Ok(Self {
            hold,
            next_topo,
            fleet_key,
        })
    }

    /// The current head of the ACL log — what a locally-authored Notice pins
    /// (§8.3).
    pub async fn acl_head(&self) -> Result<RecordID, ReplicaError> {
        self.acl_index().await?.head().ok_or(ReplicaError::NoAcl)
    }

    /// The folded ACL state, materialized from the hold (§8). Fresh each
    /// call, so it always reflects the latest synced records.
    pub async fn acl_index(&self) -> Result<AclIndex, ReplicaError> {
        Ok(self.hold.acl_index().await?)
    }

    /// Read access to the underlying hold (heads, track queries, records).
    pub fn hold(&self) -> &TerritoryHold {
        &self.hold
    }

    fn take_topo(&self) -> i64 {
        self.next_topo.fetch_add(1, Ordering::SeqCst)
    }

    // ---- ACL log sync (§8, linear ordered log) -------------------------

    /// How many ACL records this replica holds — a peer's position in the
    /// linear log.
    pub async fn acl_len(&self) -> Result<u64, ReplicaError> {
        Ok(self.hold.acl_records().await?.len() as u64)
    }

    /// The ordered ACL records at or after log position `from` — the suffix a
    /// peer that is behind needs, each carrying its order proof (§8.4).
    pub async fn acl_suffix(&self, from: u64) -> Result<Vec<OrderedRecord>, ReplicaError> {
        Ok(self
            .hold
            .ordered_records()
            .await?
            .into_iter()
            .skip(from as usize)
            .collect())
    }

    /// Append newly-received **ordered** ACL records to the log (§8.4). Each
    /// is verified against the fleet key (when this Territory is fleeted),
    /// then folded (chaining + authority) before it is persisted. A record
    /// that fails either check stops ingestion — the linear log cannot skip a
    /// gap. Already-held records are skipped, so re-delivery is idempotent.
    pub async fn ingest_ordered(&self, incoming: Vec<OrderedRecord>) -> Result<(), ReplicaError> {
        let existing = self.hold.acl_records().await?;
        let mut fold = AclIndex::from_ordered(&existing)?;
        let mut known: HashSet<RecordID> = existing
            .iter()
            .map(RecordID::of)
            .collect::<Result<_, _>>()?;
        let mut position = existing.len() as u64;
        for ordered in incoming {
            let id = RecordID::of(&ordered.record)?;
            if known.contains(&id) {
                continue; // already held
            }
            // Ordering: a fleeted Territory rejects records lacking a valid
            // orderer proof (§8.4); the Root is exempt.
            if let Some(fleet_key) = &self.fleet_key
                && ordered.verify(fleet_key).is_err()
            {
                break;
            }
            // Content: chaining + authority. A break means the record does
            // not extend our head, and nothing after it can apply either.
            if fold.push(&ordered.record).is_err() {
                break;
            }
            self.hold
                .append_acl(&ordered.record, position, ordered.proof.as_ref())
                .await?;
            known.insert(id);
            position += 1;
        }
        Ok(())
    }

    /// Append newly-received bare ACL records (no proofs) — the local /
    /// unfleeted path, e.g. handing records directly between vessels in a
    /// test. Wraps each as an unproven [`OrderedRecord`]; on a fleeted
    /// Replica these would be rejected, which is the point.
    pub async fn ingest_acl(&self, incoming: Vec<ACLRecord>) -> Result<(), ReplicaError> {
        let ordered = incoming
            .into_iter()
            .map(|record| OrderedRecord {
                record,
                proof: None,
            })
            .collect();
        self.ingest_ordered(ordered).await
    }

    /// Append a locally-authored ACL record with its (optional) order proof,
    /// fold-validating it first — unlike ingestion, an invalid record (broken
    /// chain, insufficient authority) is returned as an **error**, not
    /// silently dropped, so a membership operation can report why it was
    /// refused. `Root` and single-writer records pass `None`; an
    /// orderer-proven record passes its proof.
    pub async fn apply_local(
        &self,
        record: &ACLRecord,
        proof: Option<&OrderProof>,
    ) -> Result<RecordID, ReplicaError> {
        let existing = self.hold.acl_records().await?;
        let mut fold = AclIndex::from_ordered(&existing)?;
        fold.push(record)?; // chaining + authority; surfaces the reject
        let position = existing.len() as u64;
        self.hold.append_acl(record, position, proof).await?;
        RecordID::of(record).map_err(ReplicaError::from)
    }

    /// Order a locally-chained ACL record: assign it the next position and
    /// countersign with the fleet key (§8.4). Only an orderer-role Station
    /// holds the fleet **signing** key, so only it can call this. Returns the
    /// proof; the caller persists the ordered record. `None` if the record
    /// does not chain from the current head.
    pub async fn order(
        &self,
        fleet_signing: &ed25519_dalek::SigningKey,
        record: &ACLRecord,
    ) -> Result<Option<OrderProof>, ReplicaError> {
        let existing = self.hold.acl_records().await?;
        let mut fold = AclIndex::from_ordered(&existing)?;
        if fold.push(record).is_err() {
            return Ok(None); // does not chain / unauthorized
        }
        let position = existing.len() as u64;
        let id = RecordID::of(record)?;
        let proof = OrderProof::sign(fleet_signing, id, position);
        self.hold.append_acl(record, position, Some(&proof)).await?;
        Ok(Some(proof))
    }

    /// Apply a batch of received Notices: validate each (§5.4), append the
    /// ready ones, and retry the pending (orphan-queued) ones until no
    /// further progress. Duplicates are skipped, so re-delivery is
    /// idempotent. Rejected Notices are dropped (terminal, non-gossiped).
    pub async fn ingest(&self, incoming: Vec<SignedNotice>) -> Result<(), ReplicaError> {
        let acl = self.hold.acl_index().await?;
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
                match validate(&notice, &acl, &store) {
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
            tracing::warn!("replica notice ingest failed: {e}");
        }
    }

    async fn acl_len(&self) -> u64 {
        Replica::acl_len(self).await.unwrap_or(0)
    }

    async fn acl_suffix(&self, from: u64) -> Vec<OrderedRecord> {
        Replica::acl_suffix(self, from).await.unwrap_or_default()
    }

    async fn ingest_ordered_acl(&self, records: Vec<OrderedRecord>) {
        if let Err(e) = Replica::ingest_ordered(self, records).await {
            tracing::warn!("replica acl ingest failed: {e}");
        }
    }
}
