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
    /// Arrival order on THIS device: the seq an embedded peer serves
    /// (§4.2 paging), assigned by SQLite, monotonic, never reused.
    #[sea_orm(primary_key)]
    pub local_seq: i64,

    /// OpID — group-level idempotency key (UNIQUE).
    #[sea_orm(unique)]
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
    /// efficiently.
    #[sea_orm(indexed)]
    pub hlc: i64,

    /// Serialized Vec<Maneuver>. NULL on carrier rows (accepted via
    /// serving, not yet applied): parsing checkpoints is the domain
    /// plane's job, at apply time.
    pub ops: Option<Json>,

    /// Vec<MarkID>. NULL on carrier rows, like `ops`.
    pub touched_marks: Option<Json>,

    /// The wire envelope, byte-verbatim — what an embedded peer serves,
    /// so this device can carry envelopes it cannot parse. NULL only on
    /// rows predating V0008; readers reconstruct those from `ops`.
    pub payload: Option<String>,

    /// When this device applied the group. NULL = stored but not
    /// applied (a carrier row awaiting apply_pending).
    pub applied_at: Option<DateTime>,

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
