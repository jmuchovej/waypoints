use domain::terrain::{Terrain, TerrainKind};
use sea_orm::{
    ActiveModelBehavior, ActiveValue::Set, DeriveEntityModel, DerivePrimaryKey, DeriveRelation,
    EntityTrait, EnumIter, PrimaryKeyTrait, prelude::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

    /// What kind of edge is this? Stored as "child", "reference", or "custom".
    #[sea_orm(indexed)]
    pub kind: String,

    /// Lexicographic ordering string (only meaningful for certain edge kinds)
    pub order: Option<String>,

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
        let kind = match model.kind.as_str() {
            "child" => TerrainKind::Child,
            "reference" => TerrainKind::Reference,
            _ => TerrainKind::Custom,
        };

        let mut terrain = Terrain::builder()
            .with_src_id(model.src_id.into())
            .with_dst_id(model.dst_id.into())
            .with_kind(kind)
            .build();

        terrain.order = model.order;
        terrain.extra = model.extra;

        terrain
    }
}

impl From<Terrain> for ActiveModel {
    fn from(terrain: Terrain) -> Self {
        let kind_str = match terrain.kind {
            TerrainKind::Child => "child",
            TerrainKind::Reference => "reference",
            TerrainKind::Custom => "custom",
        };

        ActiveModel {
            src_id: Set(terrain.src_id.inner()),
            dst_id: Set(terrain.dst_id.inner()),
            kind: Set(kind_str.to_string()),
            order: Set(terrain.order),
            extra: Set(terrain.extra),
        }
    }
}
