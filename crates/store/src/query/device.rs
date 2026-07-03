use domain::sync::{AccountID, DeviceID, HLC, RealmID};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    sea_query::Expr,
};

use crate::entities::{ActiveDevice, DeviceColumn, DeviceModel, Devices};
use crate::error::{StoreError, StoreResult};

/// Fetch the local install's device row. The table holds exactly one row
/// owned by this install; other devices are only known indirectly through
/// `sync_log.device_id`.
pub async fn get_local(db: &DatabaseConnection) -> StoreResult<Option<DeviceModel>> {
    Ok(Devices::find().one(db).await?)
}

/// Mint the local install's device row. Called once at first sync
/// bootstrap; `last_hlc` starts at zero.
pub async fn insert(
    db: &DatabaseConnection,
    device_id: DeviceID,
    account_id: AccountID,
    realm_id: RealmID,
    friendly_name: &str,
) -> StoreResult<()> {
    ActiveDevice {
        device_id: Set(device_id.inner()),
        account_id: Set(account_id.inner()),
        realm_id: Set(realm_id.inner()),
        friendly_name: Set(friendly_name.to_string()),
        last_hlc: Set(0),
        alias_of: Set(None),
        created_at: Set(chrono::Local::now().naive_utc()),
        retired_at: Set(None),
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Persist the local HLC high-water mark. Called on every tick/receive so
/// the clock survives restarts.
pub async fn set_last_hlc(
    db: &DatabaseConnection,
    device_id: DeviceID,
    hlc: HLC,
) -> StoreResult<()> {
    let result = Devices::update_many()
        .col_expr(DeviceColumn::LastHlc, Expr::value(hlc.inner()))
        .filter(DeviceColumn::DeviceId.eq(device_id.inner()))
        .exec(db)
        .await?;
    if result.rows_affected == 0 {
        return Err(StoreError::Custom(format!(
            "device row missing for {device_id}"
        )));
    }
    Ok(())
}
