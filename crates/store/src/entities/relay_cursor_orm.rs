use async_trait::async_trait;
use sea_orm::{
    ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EnumIter,
    PrimaryKeyTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Fetch cursor for one (relay, realm) source. `seq` values are assigned
/// by each relay in its own arrival order — a cursor is meaningful only
/// for the relay that issued it, hence the composite key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "relay_cursors")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub relay_id: String,

    #[sea_orm(primary_key, auto_increment = false)]
    pub realm_id: Uuid,

    pub last_seq: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {}
