//! [`TerritoryHold`]: one bedrock database per Territory, holding
//! that Territory's Notices and ACL log.
//!
//! What a hold does and does not do: it *persists and indexes* records and
//! *materializes* the folded ACL state for validation. It does not itself
//! run `validate()` — the caller drives that, deciding accept/pending/drop
//! (a Station and a Vessel make the same validation call but attach
//! different follow-on duties). A hold refuses to persist a
//! Notice whose bytes do not decode, but "well-formed bytes" is the only
//! judgment it makes; authority is the validator's job.

use std::collections::{BTreeMap, BTreeSet};

use bedrock::{Database, Filter, Query};
use outpost_protocol::acl::{ACLRecord, OrderProof, OrderedRecord};
use outpost_protocol::{NoticeID, NoticeStore, RecordID, SignedNotice, TrackID};

use crate::acl_index::AclIndex;
use crate::error::HoldError;
use crate::rows::{AclRow, NoticeKey, NoticeRow, PendingRow, RecordKey};

/// A Notice as it comes back from the store: the canonical bytes plus the
/// cheap denormalized facts the store already indexed.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredNotice {
    pub id: NoticeID,
    pub track: TrackID,
    pub topo: i64,
    pub is_head: bool,
    pub notice: SignedNotice,
}

impl StoredNotice {
    fn from_row(row: NoticeRow) -> Result<Self, HoldError> {
        let notice = SignedNotice::decode(&row.bytes)?;
        Ok(Self {
            id: NoticeID::from_bytes(row.id.0),
            track: TrackID::from(NoticeID::from_bytes(row.track)),
            topo: row.topo,
            is_head: row.is_head,
            notice,
        })
    }
}

/// Per-Territory record storage.
pub struct TerritoryHold {
    db: Database,
}

impl TerritoryHold {
    /// Open (creating if absent) a Territory's database at `path`, or
    /// `":memory:"`. Idempotent: re-opening ensures the collections exist.
    pub async fn open(path: &str) -> Result<Self, HoldError> {
        let db = Database::open(path).await?;
        db.ensure_collection::<NoticeRow>().await?;
        db.ensure_collection::<AclRow>().await?;
        db.ensure_collection::<PendingRow>().await?;
        Ok(Self { db })
    }

    // ---- ACL log -------------------------------------------------------

    /// Append an ACL record at `position` with its orderer proof. The
    /// caller has already validated it against the fold; the hold persists the
    /// bytes, position, and proof that let a later fold replay and re-verify
    /// them. `proof` is `None` only for the self-authenticating `Root`.
    pub async fn append_acl(
        &self,
        record: &ACLRecord,
        position: u64,
        proof: Option<&OrderProof>,
    ) -> Result<RecordID, HoldError> {
        let id = RecordID::of(record)?;
        let proof_bytes = match proof {
            Some(p) => postcard::to_allocvec(p)?,
            None => Vec::new(),
        };
        self.db
            .insert(AclRow {
                id: RecordKey(*id.as_bytes()),
                position: position as i64,
                bytes: record.encode()?.to_vec(),
                proof: proof_bytes,
            })
            .await?;
        Ok(id)
    }

    /// Every ACL record, in log order (proofs dropped — for the fold, which
    /// re-verifies content, not ordering).
    pub async fn acl_records(&self) -> Result<Vec<ACLRecord>, HoldError> {
        let rows = self
            .db
            .query::<AclRow>(&Query::new().sort_asc("position"))
            .await?;
        rows.iter()
            .map(|row| ACLRecord::decode(&row.bytes).map_err(HoldError::from))
            .collect()
    }

    /// Every ACL record with its order proof, in log order — what sync
    /// transfers so a peer can re-verify ordering.
    pub async fn ordered_records(&self) -> Result<Vec<OrderedRecord>, HoldError> {
        let rows = self
            .db
            .query::<AclRow>(&Query::new().sort_asc("position"))
            .await?;
        rows.iter()
            .map(|row| {
                let record = ACLRecord::decode(&row.bytes)?;
                let proof = if row.proof.is_empty() {
                    None
                } else {
                    Some(postcard::from_bytes::<OrderProof>(&row.proof)?)
                };
                Ok(OrderedRecord { record, proof })
            })
            .collect()
    }

    /// Materialize the folded ACL state (the `ACLStateProvider`), ready
    /// to hand to `validate()`.
    pub async fn acl_index(&self) -> Result<AclIndex, HoldError> {
        AclIndex::from_ordered(&self.acl_records().await?)
    }

    // ---- Notices -------------------------------------------------------

    /// Persist an already-validated Notice as a head, and clear the head
    /// flag on any parent it now covers. `topo` is the caller-assigned
    /// insertion cursor (monotonic within the hold). Returns the Notice's
    /// content address.
    pub async fn append_notice(
        &self,
        notice: &SignedNotice,
        track: TrackID,
        topo: i64,
    ) -> Result<NoticeID, HoldError> {
        let envelope = notice.parse_and_verify()?;
        let id = NoticeID::of(notice)?;
        let header = &envelope.header;

        self.db
            .insert(NoticeRow {
                id: NoticeKey(*id.as_bytes()),
                track: track.root_notice().as_bytes().to_owned(),
                topo,
                is_head: true,
                is_snapshot: header.is_snapshot,
                acl_head: *header.acl_head.as_bytes(),
                bytes: notice.encode()?.to_vec(),
            })
            .await?;

        // Any parent this Notice descends from is no longer a head.
        for parent in &header.parents {
            self.clear_head(*parent).await?;
        }
        Ok(id)
    }

    async fn clear_head(&self, id: NoticeID) -> Result<(), HoldError> {
        if let Some(mut row) = self.db.get::<NoticeRow>(&NoticeKey(*id.as_bytes())).await?
            && row.is_head
        {
            row.is_head = false;
            self.db.update(row).await?;
        }
        Ok(())
    }

    /// Fetch a stored Notice by content address.
    pub async fn get_notice(&self, id: NoticeID) -> Result<Option<StoredNotice>, HoldError> {
        match self.db.get::<NoticeRow>(&NoticeKey(*id.as_bytes())).await? {
            Some(row) => Ok(Some(StoredNotice::from_row(row)?)),
            None => Ok(None),
        }
    }

    /// The head Notices of a Track: those with no accepted children.
    /// Multiple heads = unmerged concurrent edits.
    pub async fn track_heads(&self, track: TrackID) -> Result<Vec<StoredNotice>, HoldError> {
        let rows = self
            .db
            .query::<NoticeRow>(&Query::new().filter(Filter::all([
                Filter::eq("track", track.root_notice().as_bytes().to_owned()),
                Filter::eq("is_head", true),
            ])))
            .await?;
        rows.into_iter().map(StoredNotice::from_row).collect()
    }

    /// The whole Territory's `(Track → head ids)` map — the input to a
    /// [`TallyDigest`](outpost_protocol::sync::TallyDigest).
    pub async fn all_heads(&self) -> Result<BTreeMap<TrackID, BTreeSet<NoticeID>>, HoldError> {
        let rows = self
            .db
            .query::<NoticeRow>(&Query::new().filter(Filter::eq("is_head", true)))
            .await?;
        let mut heads: BTreeMap<TrackID, BTreeSet<NoticeID>> = BTreeMap::new();
        for row in rows {
            let stored = StoredNotice::from_row(row)?;
            heads.entry(stored.track).or_default().insert(stored.id);
        }
        Ok(heads)
    }

    /// Every Notice belonging to one Track (its whole DAG, not just heads)
    /// — a resupply offer for a divergent Track.
    pub async fn track_notices(&self, track: TrackID) -> Result<Vec<StoredNotice>, HoldError> {
        let rows = self
            .db
            .query::<NoticeRow>(
                &Query::new()
                    .filter(Filter::eq(
                        "track",
                        track.root_notice().as_bytes().to_owned(),
                    ))
                    .sort_asc("topo"),
            )
            .await?;
        rows.into_iter().map(StoredNotice::from_row).collect()
    }

    /// Every Notice id the hold currently holds — the `have` set a resupply
    /// filters an offer against.
    pub async fn held_ids(&self) -> Result<BTreeSet<NoticeID>, HoldError> {
        Ok(self
            .notices_after(-1)
            .await?
            .into_iter()
            .map(|s| s.id)
            .collect())
    }

    /// Every Notice with topo cursor greater than `after`, in cursor order
    /// — the resupply "everything after X" scan. `after = -1` yields
    /// the whole hold.
    pub async fn notices_after(&self, after: i64) -> Result<Vec<StoredNotice>, HoldError> {
        let rows = self
            .db
            .query::<NoticeRow>(
                &Query::new()
                    .filter(Filter::gt("topo", after))
                    .sort_asc("topo"),
            )
            .await?;
        rows.into_iter().map(StoredNotice::from_row).collect()
    }

    /// The largest topo cursor in the hold, or -1 if empty. The caller uses
    /// `max_topo() + 1` as the next insertion cursor.
    pub async fn max_topo(&self) -> Result<i64, HoldError> {
        let rows = self
            .db
            .query::<NoticeRow>(&Query::new().sort_desc("topo").limit(1))
            .await?;
        Ok(rows.first().map_or(-1, |row| row.topo))
    }

    // ---- Orphan queue --------------------------------------------------

    /// Park a Notice that is waiting on a still-missing `parent`
    /// (`Pending`). Keyed by the Notice id; one row per waiting Notice.
    pub async fn park_pending(
        &self,
        notice: &SignedNotice,
        parent: NoticeID,
    ) -> Result<(), HoldError> {
        let id = NoticeID::of(notice)?;
        self.db
            .insert(PendingRow {
                id: NoticeKey(*id.as_bytes()),
                parent: *parent.as_bytes(),
                bytes: notice.encode()?.to_vec(),
            })
            .await?;
        Ok(())
    }

    /// Notices parked waiting on `parent` — the queue to re-validate when
    /// `parent` finally arrives.
    pub async fn pending_on(&self, parent: NoticeID) -> Result<Vec<SignedNotice>, HoldError> {
        let rows = self
            .db
            .query::<PendingRow>(
                &Query::new().filter(Filter::eq("parent", parent.as_bytes().to_owned())),
            )
            .await?;
        rows.iter()
            .map(|row| SignedNotice::decode(&row.bytes).map_err(HoldError::from))
            .collect()
    }

    /// Remove a Notice from the orphan queue once it has been accepted.
    pub async fn unpark(&self, id: NoticeID) -> Result<(), HoldError> {
        self.db
            .delete::<PendingRow>(&NoticeKey(*id.as_bytes()))
            .await?;
        Ok(())
    }
}

/// A synchronous presence view over a hold's Notices, for `validate()`.
/// Built by loading the ids and pins the validator will consult — the
/// async store cannot satisfy the sync [`NoticeStore`] trait directly.
#[derive(Debug, Default, Clone)]
pub struct NoticeIndex {
    acl_heads: std::collections::BTreeMap<NoticeID, RecordID>,
}

impl NoticeIndex {
    /// Snapshot the (id → acl_head) map from the hold's Notices, so the
    /// validator can check monotonicity against accepted parents.
    pub async fn snapshot(hold: &TerritoryHold) -> Result<Self, HoldError> {
        let mut acl_heads = std::collections::BTreeMap::new();
        for stored in hold.notices_after(-1).await? {
            let head = stored.notice.parse_and_verify()?.header.acl_head;
            acl_heads.insert(stored.id, head);
        }
        Ok(Self { acl_heads })
    }
}

impl NoticeStore for NoticeIndex {
    fn contains(&self, notice: NoticeID) -> bool {
        self.acl_heads.contains_key(&notice)
    }

    fn acl_head_of(&self, notice: NoticeID) -> Option<RecordID> {
        self.acl_heads.get(&notice).copied()
    }
}
