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
use outpost_protocol::{HLC, NoticeID};

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
}

/// Durably enqueue a payload for publication. `seq` assignment assumes a
/// single writer (the editor serializes mutations).
pub async fn enqueue_publish(store: &Store, payload: Vec<u8>) -> StoreResult<i64> {
    let seq = store
        .db()
        .query::<SyncOutbox>(&bedrock::Query::new().sort_desc("seq").limit(1))
        .await?
        .first()
        .map(|row| row.seq + 1)
        .unwrap_or(0);
    store.db().insert(SyncOutbox { seq, payload }).await?;
    Ok(seq)
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
