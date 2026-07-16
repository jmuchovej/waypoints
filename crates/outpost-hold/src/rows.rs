//! Bedrock collections backing a Territory (§11.1). Each row keeps the
//! canonical raw bytes plus denormalized header columns for indexing. The
//! `id` columns are native binary keys (the [`u8; 32]` `Field` impl) — no
//! text-encoding detour for content addresses.

use bedrock::{Collection, Scalar};

/// A Notice's 32-byte content address as a binary primary key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Scalar)]
pub struct NoticeKey(pub [u8; 32]);

/// An ACL record's 32-byte content address as a binary primary key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Scalar)]
pub struct RecordKey(pub [u8; 32]);

/// One stored Notice. `bytes` is the object of record; every other column
/// is a denormalized copy of a header field, present only for indexing
/// (§11.1) and never trusted for a security decision.
#[derive(Debug, Clone, PartialEq, Collection)]
#[bedrock(index("track", "topo"))]
pub struct NoticeRow {
    #[bedrock(id)]
    pub id: NoticeKey,
    /// Track root's NoticeID — the Track this Notice belongs to.
    #[bedrock(index)]
    pub track: [u8; 32],
    /// Total insertion order within this hold: the topological cursor for
    /// "everything after K" range scans (§6.2).
    pub topo: i64,
    /// True ⇒ some accepted Notice lists this one as a parent. A Notice
    /// with no children is a head (§6.1); heads are `is_head = true`.
    #[bedrock(index)]
    pub is_head: bool,
    pub is_snapshot: bool,
    /// Denormalized `acl_head` pin, for the monotonicity check without
    /// re-parsing every parent.
    pub acl_head: [u8; 32],
    /// The canonical encoded `SignedNotice` — the object of record.
    pub bytes: Vec<u8>,
}

/// One stored ACL record, in log order. `position` is the orderer-assigned
/// index (§8.4); the fold replays them in that order.
#[derive(Debug, Clone, PartialEq, Collection)]
pub struct AclRow {
    #[bedrock(id)]
    pub id: RecordKey,
    #[bedrock(index)]
    pub position: i64,
    /// The canonical encoded `ACLRecord`.
    pub bytes: Vec<u8>,
    /// The orderer's countersignature (§8.4), postcard-encoded `OrderProof`.
    /// Empty for the `Root`, which is self-authenticating (position 0).
    pub proof: Vec<u8>,
}

/// A parent edge awaiting its parent: the child cannot be accepted until
/// `parent` arrives (§5.4 `Pending`). Keyed by the missing parent so the
/// arrival of a Notice can look up who was waiting on it.
#[derive(Debug, Clone, PartialEq, Collection)]
pub struct PendingRow {
    #[bedrock(id)]
    pub id: NoticeKey,
    /// The still-missing parent this child waits on.
    #[bedrock(index)]
    pub parent: [u8; 32],
    /// The canonical encoded `SignedNotice` of the waiting child.
    pub bytes: Vec<u8>,
}
