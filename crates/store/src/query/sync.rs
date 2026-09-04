//! Sync-owned persistence: the winning arbitration key per fold target and
//! the incremental fold cursor.
//!
//! This module is the reason `store` carries a scoped `outpost-protocol`
//! dependency — the winning key names `NoticeID` and `HLC`. The dependency
//! is confined to these sync-owned tables; nothing else in `store` may name
//! protocol types.
//!
//! `target` is the canonical string cartograph derives from a
//! `ManeuverTarget`; it is opaque here.

use crate::{Store, StoreResult};
use outpost_protocol::{HLC, NoticeID, TrackID};

/// The winning arbitration key currently materialized into one fold target.
#[derive(Debug, Clone, PartialEq, bedrock::Collection)]
pub struct SyncWinner {
    #[bedrock(id)]
    pub target: String,
    /// Packed HLC of the winning key.
    pub hlc: i64,
    /// Author verifying-key bytes of the winning key.
    pub author: [u8; 32],
    /// Content address of the winning key's Notice.
    pub notice: [u8; 32],
}

/// Single-row incremental fold state; `id` is always 0.
#[derive(Debug, Clone, PartialEq, bedrock::Collection)]
pub struct SyncFoldState {
    #[bedrock(id)]
    pub id: i64,
    /// The topo cursor: every Notice with `topo <= cursor` has been offered
    /// to the fold. `-1` ⇒ nothing folded yet.
    pub cursor: i64,
}

/// One queued publication: a `waypoints.page.v1` plaintext (an encoded op
/// batch) awaiting authorship as a Notice. Rows publish in `seq` order and
/// are deleted only on confirmed authorship, so a crash between local apply
/// and publication leaves the edit queued rather than dropped.
#[derive(Debug, Clone, PartialEq, bedrock::Collection)]
pub struct SyncOutbox {
    #[bedrock(id)]
    pub seq: i64,
    pub payload: Vec<u8>,
    /// The Landmark this batch concerns (lowercase-uuid `MarkID`), chosen
    /// deterministically by the enqueuer — publication picks that
    /// Landmark's Track. Empty ⇒ unknown; the Notice roots a new Track.
    pub landmark: String,
}

/// Durably enqueue a payload for publication. `seq` assignment assumes a
/// single writer (the editor serializes mutations).
pub async fn enqueue_publish(
    store: &Store,
    payload: Vec<u8>,
    landmark: String,
) -> StoreResult<i64> {
    let seq = store
        .db()
        .query::<SyncOutbox>(&bedrock::Query::new().sort_desc("seq").limit(1))
        .await?
        .first()
        .map(|row| row.seq + 1)
        .unwrap_or(0);
    store
        .db()
        .insert(SyncOutbox {
            seq,
            payload,
            landmark,
        })
        .await?;
    Ok(seq)
}

/// The write-side `MarkID → TrackID` index: which Track a Landmark's
/// history occupies. Consulted only when choosing the parents of a Notice
/// being authored — never on the fold path — so loss, staleness, or
/// corruption degrades partitioning, never graph state.
#[derive(Debug, Clone, PartialEq, bedrock::Collection)]
pub struct SyncTrack {
    /// Lowercase-uuid `MarkID` of the Landmark.
    #[bedrock(id)]
    pub mark: String,
    /// Root-Notice bytes of the Track.
    pub track: [u8; 32],
}

/// The Track a Landmark's history occupies, if this device has authored
/// (and indexed) one.
pub async fn track_of(store: &Store, mark: &str) -> StoreResult<Option<TrackID>> {
    Ok(store
        .db()
        .get::<SyncTrack>(&mark.to_owned())
        .await?
        .map(|row| TrackID::from(NoticeID::from_bytes(row.track))))
}

/// Record the Track a Landmark's history occupies.
pub async fn set_track(store: &Store, mark: &str, track: TrackID) -> StoreResult<()> {
    store
        .db()
        .upsert::<SyncTrack>(SyncTrack {
            mark: mark.to_owned(),
            track: *track.root_notice().as_bytes(),
        })
        .await?;
    Ok(())
}

/// Every queued publication, oldest first.
pub async fn publish_queue(store: &Store) -> StoreResult<Vec<SyncOutbox>> {
    Ok(store
        .db()
        .query::<SyncOutbox>(&bedrock::Query::new().sort_asc("seq"))
        .await?)
}

/// Remove one queued publication after its Notice's authorship is confirmed.
pub async fn clear_published(store: &Store, seq: i64) -> StoreResult<()> {
    store.db().delete::<SyncOutbox>(&seq).await?;
    Ok(())
}

/// One operation deferred on a Mark whose creation has not folded yet.
/// Keyed rows release — in arbitration-key order — when the missing
/// `CreateMark` folds; until then they are neither applied nor dropped.
#[derive(Debug, Clone, PartialEq, bedrock::Collection)]
pub struct SyncDeferred {
    #[bedrock(id)]
    pub seq: i64,
    /// Lowercase-uuid `MarkID` the op waits on.
    #[bedrock(index)]
    pub missing: String,
    /// Arbitration-key components of the carrying Notice.
    pub hlc: i64,
    pub author: [u8; 32],
    pub notice: [u8; 32],
    /// The op's index within its Notice's batch.
    pub batch_index: i64,
    /// The serialized op (`waypoints.page.v1` single-op encoding).
    pub op: Vec<u8>,
}

/// Defer an op on a missing Mark.
pub async fn defer_op(
    store: &Store,
    missing: &str,
    key: StoredKey,
    batch_index: i64,
    op: Vec<u8>,
) -> StoreResult<i64> {
    let seq = store
        .db()
        .query::<SyncDeferred>(&bedrock::Query::new().sort_desc("seq").limit(1))
        .await?
        .first()
        .map(|row| row.seq + 1)
        .unwrap_or(0);
    store
        .db()
        .insert(SyncDeferred {
            seq,
            missing: missing.to_owned(),
            hlc: key.hlc.inner(),
            author: key.author,
            notice: *key.notice.as_bytes(),
            batch_index,
            op,
        })
        .await?;
    Ok(seq)
}

/// Every op deferred on `missing`.
pub async fn deferred_on(store: &Store, missing: &str) -> StoreResult<Vec<SyncDeferred>> {
    Ok(store
        .db()
        .query::<SyncDeferred>(
            &bedrock::Query::new()
                .filter(bedrock::Filter::eq("missing", missing.to_owned()))
                .sort_asc("seq"),
        )
        .await?)
}

/// Remove one deferred op (it is being released back into the fold).
pub async fn remove_deferred(store: &Store, seq: i64) -> StoreResult<()> {
    store.db().delete::<SyncDeferred>(&seq).await?;
    Ok(())
}

/// How many ops are currently deferred — the observability hook for
/// permanently unresolved references.
pub async fn deferred_count(store: &Store) -> StoreResult<u64> {
    Ok(store.db().count::<SyncDeferred>().await?)
}

/// A winning key, decomposed into its stored components. The sequence
/// `(hlc, author, notice)` is the arbitration order; comparisons happen in
/// cartograph's key type, never here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredKey {
    pub hlc: HLC,
    pub author: [u8; 32],
    pub notice: NoticeID,
}

/// The persisted winning key for `target`, if any op ever materialized it.
pub async fn winner(store: &Store, target: &str) -> StoreResult<Option<StoredKey>> {
    Ok(store
        .db()
        .get::<SyncWinner>(&target.to_owned())
        .await?
        .map(|row| StoredKey {
            hlc: HLC(row.hlc),
            author: row.author,
            notice: NoticeID::from_bytes(row.notice),
        }))
}

/// Record `key` as the winning key for `target`.
pub async fn set_winner(store: &Store, target: &str, key: StoredKey) -> StoreResult<()> {
    store
        .db()
        .upsert::<SyncWinner>(SyncWinner {
            target: target.to_owned(),
            hlc: key.hlc.inner(),
            author: key.author,
            notice: *key.notice.as_bytes(),
        })
        .await?;
    Ok(())
}

/// The incremental fold cursor: the greatest hold `topo` already offered to
/// the fold, `-1` before the first fold.
pub async fn fold_cursor(store: &Store) -> StoreResult<i64> {
    Ok(store
        .db()
        .get::<SyncFoldState>(&0)
        .await?
        .map(|row| row.cursor)
        .unwrap_or(-1))
}

/// Advance the incremental fold cursor.
pub async fn set_fold_cursor(store: &Store, cursor: i64) -> StoreResult<()> {
    store
        .db()
        .upsert::<SyncFoldState>(SyncFoldState { id: 0, cursor })
        .await?;
    Ok(())
}
