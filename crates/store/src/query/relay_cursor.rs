use domain::sync::RealmID;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::entities::{ActiveRelayCursor, RelayCursorColumn, RelayCursors};
use crate::error::StoreResult;

/// Last seq fetched from this (relay, realm) source. 0 = never fetched;
/// the relay's seqs start at 1.
pub async fn get(
    db: &DatabaseConnection,
    relay_id: &str,
    realm_id: RealmID,
) -> StoreResult<i64> {
    let row = RelayCursors::find_by_id((relay_id.to_string(), realm_id.inner()))
        .one(db)
        .await?;
    Ok(row.map(|r| r.last_seq).unwrap_or(0))
}

pub async fn set(
    db: &DatabaseConnection,
    relay_id: &str,
    realm_id: RealmID,
    last_seq: i64,
) -> StoreResult<()> {
    RelayCursors::insert(ActiveRelayCursor {
        relay_id: Set(relay_id.to_string()),
        realm_id: Set(realm_id.inner()),
        last_seq: Set(last_seq),
    })
    .on_conflict(
        OnConflict::columns([RelayCursorColumn::RelayId, RelayCursorColumn::RealmId])
            .update_column(RelayCursorColumn::LastSeq)
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}
