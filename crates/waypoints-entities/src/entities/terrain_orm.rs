use sea_orm::{
    ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EntityTrait,
    EnumIter, PrimaryKeyTrait, prelude::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use waypoints_domain::terrain::Terrain;

/// Graph edges representing relationships between blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "terrains", rename_all = "kebab-case")]
pub struct Model {
    /// The directed edge this block is coming from
    #[sea_orm(primary_key)]
    pub src_id: Uuid,

    /// The directed edge this block is going to
    #[sea_orm(primary_key)]
    pub dst_id: Uuid,

    /// Extra metadata that belongs to this particular edge
    pub extra: Option<Json>,
}

use super::{MarkColumn, Marks};
use Column::{DstId, SrcId};

#[derive(Debug, Copy, Clone, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "Marks", from = "SrcId", to = "MarkColumn::Id")]
    SrcMark,
    #[sea_orm(belongs_to = "Marks", from = "DstId", to = "MarkColumn::Id")]
    DstMark,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for Terrain {
    fn from(model: Model) -> Self {
        let mut builder = Terrain::builder()
            .with_src_id(model.src_id.into())
            .with_dst_id(model.dst_id.into());

        if let Some(extra) = model.extra {
            builder = builder.with_extra(extra);
        }

        let terrain: Terrain = builder.build();

        terrain
    }
}
