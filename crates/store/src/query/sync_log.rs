use domain::Json;
use domain::sync::{AccountID, DeviceID, HLC, OpID, RealmID};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
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

/// A new `sync_log` row. `ops` and `touched_marks` are opaque JSON — the
/// store never interprets them (the wire types live in cartograph).
#[derive(Debug, Clone)]
pub struct SyncLogEntry {
    pub group_id: OpID,
    pub device_id: DeviceID,
    pub account_id: AccountID,
    pub realm_id: RealmID,
    pub hlc: HLC,
    pub ops: Json,
    pub touched_marks: Json,
    pub origin: SyncOrigin,
}

/// Idempotency probe: has this group already been persisted?
pub async fn contains(db: &DatabaseConnection, group_id: OpID) -> StoreResult<bool> {
    Ok(SyncLog::find_by_id(group_id.inner())
        .one(db)
        .await?
        .is_some())
}

pub async fn insert(db: &DatabaseConnection, entry: SyncLogEntry) -> StoreResult<()> {
    ActiveSyncLog {
        group_id: Set(entry.group_id.inner()),
        device_id: Set(entry.device_id.inner()),
        account_id: Set(entry.account_id.inner()),
        realm_id: Set(entry.realm_id.inner()),
        hlc: Set(entry.hlc.inner()),
        ops: Set(entry.ops),
        touched_marks: Set(entry.touched_marks),
        applied_at: Set(chrono::Local::now().naive_utc()),
        origin: Set(entry.origin as i32),
    }
    .insert(db)
    .await?;
    Ok(())
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
