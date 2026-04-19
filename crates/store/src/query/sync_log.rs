use domain::Json;
use domain::sync::{AccountID, DeviceID, HLC, OpID, RealmID};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::entities::{ActiveSyncLog, SyncLog, SyncLogColumn, SyncLogModel};
use crate::error::StoreResult;

/// Where a logged group came from, as recorded on THIS device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOrigin {
    /// This device produced the group.
    Local = 0,
    /// Received from a peer via the relay.
    Remote = 1,
}

/// A new, fully-applied `sync_log` row (the coordinator's write path).
/// `ops` and `touched_marks` are opaque JSON — the store never
/// interprets them (the wire types live in cartograph). `payload` is
/// the wire envelope, byte-verbatim.
#[derive(Debug, Clone)]
pub struct SyncLogEntry {
    pub group_id: OpID,
    pub device_id: DeviceID,
    pub account_id: AccountID,
    pub realm_id: RealmID,
    pub hlc: HLC,
    pub ops: Json,
    pub touched_marks: Json,
    pub payload: String,
    pub origin: SyncOrigin,
}

/// The row for a group, if this device has ever stored it. Callers
/// distinguish applied (`applied_at` set) from carrier rows.
pub async fn get(db: &DatabaseConnection, group_id: OpID) -> StoreResult<Option<SyncLogModel>> {
    Ok(SyncLog::find()
        .filter(SyncLogColumn::GroupId.eq(group_id.inner()))
        .one(db)
        .await?)
}

/// Insert an applied group (the coordinator's path: ops ran against the
/// database in the same breath).
pub async fn insert(db: &DatabaseConnection, entry: SyncLogEntry) -> StoreResult<()> {
    ActiveSyncLog {
        local_seq: NotSet,
        group_id: Set(entry.group_id.inner()),
        device_id: Set(entry.device_id.inner()),
        account_id: Set(entry.account_id.inner()),
        realm_id: Set(entry.realm_id.inner()),
        hlc: Set(entry.hlc.inner()),
        ops: Set(Some(entry.ops)),
        touched_marks: Set(Some(entry.touched_marks)),
        payload: Set(Some(entry.payload)),
        applied_at: Set(Some(chrono::Local::now().naive_utc())),
        origin: Set(entry.origin as i32),
        uploaded_at: Set(None),
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Store-without-apply: the serving write path (§4.1 accepted an upload
/// from another peer). Face fields + verbatim payload only — no parsed
/// ops, no applied_at. Idempotent on group_id; returns the row's
/// local_seq either way (the ack the §4.1 response needs).
pub async fn append_envelope(
    db: &DatabaseConnection,
    group_id: OpID,
    device_id: DeviceID,
    account_id: AccountID,
    realm_id: RealmID,
    hlc: HLC,
    payload: &str,
) -> StoreResult<i64> {
    SyncLog::insert(ActiveSyncLog {
        local_seq: NotSet,
        group_id: Set(group_id.inner()),
        device_id: Set(device_id.inner()),
        account_id: Set(account_id.inner()),
        realm_id: Set(realm_id.inner()),
        hlc: Set(hlc.inner()),
        ops: Set(None),
        touched_marks: Set(None),
        payload: Set(Some(payload.to_string())),
        applied_at: Set(None),
        origin: Set(SyncOrigin::Remote as i32),
        uploaded_at: Set(None),
    })
    .on_conflict(
        OnConflict::column(SyncLogColumn::GroupId)
            .do_nothing()
            .to_owned(),
    )
    .do_nothing()
    .exec(db)
    .await?;

    let row = get(db, group_id).await?.expect("row exists after upsert");
    Ok(row.local_seq)
}

/// Mark a carrier row applied, backfilling the parsed ops the apply
/// step just ran.
pub async fn mark_applied(
    db: &DatabaseConnection,
    group_id: OpID,
    ops: Json,
    touched_marks: Json,
) -> StoreResult<()> {
    SyncLog::update_many()
        .col_expr(SyncLogColumn::Ops, Expr::value(Some(ops)))
        .col_expr(
            SyncLogColumn::TouchedMarks,
            Expr::value(Some(touched_marks)),
        )
        .col_expr(
            SyncLogColumn::AppliedAt,
            Expr::value(Some(chrono::Local::now().naive_utc())),
        )
        .filter(SyncLogColumn::GroupId.eq(group_id.inner()))
        .exec(db)
        .await?;
    Ok(())
}

/// Carrier rows awaiting apply, arrival order.
pub async fn pending_apply(
    db: &DatabaseConnection,
    realm_id: RealmID,
    limit: u64,
) -> StoreResult<Vec<SyncLogModel>> {
    Ok(SyncLog::find()
        .filter(SyncLogColumn::RealmId.eq(realm_id.inner()))
        .filter(SyncLogColumn::AppliedAt.is_null())
        .order_by_asc(SyncLogColumn::LocalSeq)
        .limit(limit)
        .all(db)
        .await?)
}

/// The §4.2 serving window: rows with `local_seq > since`, arrival
/// order, at most `limit` (callers pass limit+1 to detect has_more).
pub async fn page_envelopes(
    db: &DatabaseConnection,
    realm_id: RealmID,
    since_seq: i64,
    limit: u64,
) -> StoreResult<Vec<SyncLogModel>> {
    Ok(SyncLog::find()
        .filter(SyncLogColumn::RealmId.eq(realm_id.inner()))
        .filter(SyncLogColumn::LocalSeq.gt(since_seq))
        .order_by_asc(SyncLogColumn::LocalSeq)
        .limit(limit)
        .all(db)
        .await?)
}

/// Highest HLC among remote-origin groups in a realm — the "give me
/// everything since X" cursor for relay fetches. `HLC::ZERO` when nothing
/// has been received yet.
pub async fn max_remote_hlc(db: &DatabaseConnection, realm_id: RealmID) -> StoreResult<HLC> {
    let row: Option<SyncLogModel> = SyncLog::find()
        .filter(SyncLogColumn::RealmId.eq(realm_id.inner()))
        .filter(SyncLogColumn::Origin.eq(SyncOrigin::Remote as i32))
        .order_by_desc(SyncLogColumn::Hlc)
        .one(db)
        .await?;
    Ok(row.map(|r| HLC(r.hlc)).unwrap_or(HLC::ZERO))
}

/// Local-origin envelopes the relay has not yet acknowledged, oldest
/// HLC first. The upload loop drains this in batches; at-least-once is
/// fine because the relay dedupes on group_id.
pub async fn pending_upload(
    db: &DatabaseConnection,
    realm_id: RealmID,
    limit: u64,
) -> StoreResult<Vec<SyncLogModel>> {
    Ok(SyncLog::find()
        .filter(SyncLogColumn::RealmId.eq(realm_id.inner()))
        .filter(SyncLogColumn::Origin.eq(SyncOrigin::Local as i32))
        .filter(SyncLogColumn::UploadedAt.is_null())
        .order_by_asc(SyncLogColumn::Hlc)
        .limit(limit)
        .all(db)
        .await?)
}

/// Record the relay's acknowledgment for the given groups.
pub async fn mark_uploaded(db: &DatabaseConnection, group_ids: &[OpID]) -> StoreResult<()> {
    if group_ids.is_empty() {
        return Ok(());
    }
    let ids: Vec<_> = group_ids.iter().map(|g| g.inner()).collect();
    SyncLog::update_many()
        .col_expr(
            SyncLogColumn::UploadedAt,
            Expr::value(chrono::Local::now().naive_utc()),
        )
        .filter(SyncLogColumn::GroupId.is_in(ids))
        .exec(db)
        .await?;
    Ok(())
}
