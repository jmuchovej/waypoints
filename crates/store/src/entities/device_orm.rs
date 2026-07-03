use async_trait::async_trait;
use sea_orm::{
    ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EnumIter,
    PrimaryKeyTrait, prelude::DateTime,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The local install's sync identity. Exactly one row belongs to this
/// install; the row is minted at first launch (Phase 5 bootstrap) and the
/// `device_id` also lives in the OS keychain so a DB wipe doesn't lose it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "devices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub device_id: Uuid,

    pub account_id: Uuid,

    /// v1: a default realm per account, minted alongside the device row.
    pub realm_id: Uuid,

    /// Defaults to the system hostname at mint time.
    pub friendly_name: String,

    /// Packed HLC of the last local tick/receive. Persisted on every
    /// clock advance and reloaded at startup.
    pub last_hlc: i64,

    /// If non-null, this device is an alias for another (a local UI
    /// concept only — historical sync_log rows are never rewritten).
    pub alias_of: Option<Uuid>,

    pub created_at: DateTime,

    /// User marked this device retired.
    pub retired_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {}
