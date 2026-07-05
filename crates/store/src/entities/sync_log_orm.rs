use async_trait::async_trait;
use sea_orm::{
    ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EnumIter,
    PrimaryKeyTrait,
    prelude::{DateTime, Json},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "sync_log")]
pub struct Model {
    /// OpID — group-level idempotency key.
    #[sea_orm(primary_key, auto_increment = false)]
    pub group_id: Uuid,

    /// Originating device (its real ID at origination time; never
    /// rewritten by device aliasing).
    #[sea_orm(indexed)]
    pub device_id: Uuid,

    pub account_id: Uuid,

    /// Workspace boundary; the relay's "since cursor" query is scoped
    /// per-realm.
    #[sea_orm(indexed)]
    pub realm_id: Uuid,

    /// Packed HLC (i64). Indexed so we can serve "all groups with hlc > X"
    /// efficiently — the relay protocol's primary query.
    #[sea_orm(indexed)]
    pub hlc: i64,

    /// Serialized Vec<AtomicOp>.
    pub ops: Json,

    /// Vec<MarkID>.
    pub touched_marks: Json,

    /// When this device persisted the group (NOT the same as HLC for
    /// remote groups; relevant for diagnostics and GC).
    pub applied_at: DateTime,

    /// 0 = local-origin (we made it), 1 = remote-origin (received from peer).
    /// Useful for "show me only my changes" or filtering during debugging.
    pub origin: i32,

    /// When the relay acknowledged this local-origin envelope.
    /// NULL = pending upload. Remote-origin rows stay NULL forever.
    pub uploaded_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {}
