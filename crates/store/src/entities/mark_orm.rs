//! Core block types and implementations.
use domain::{
    Uuid,
    mark::{Content, Landmark, Mark, MarkData, MarkID},
};
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::{
    ActiveModelBehavior, ConnectionTrait, DbErr, DeriveEntityModel, DerivePrimaryKey,
    DeriveRelation, EnumIter, PrimaryKeyTrait,
    prelude::{DateTime, Json},
};
use serde::{Deserialize, Serialize};

use crate::error::StoreError;
use crate::{StoreResult, impl_timestamps};

/// Core block structure representing any piece of content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "marks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    #[sea_orm(indexed)]
    pub mark: String,
    pub data: Json,

    pub extra: Option<Json>,

    /// When the block was created
    #[sea_orm(indexed)]
    pub created_at: DateTime,
    /// When the block was last modified
    #[sea_orm(indexed)]
    pub updated_at: DateTime,
    /// Person who created the block (optional)
    pub created_by: Option<Uuid>,
    /// Person who last modified the block (optional)
    pub updated_by: Option<Uuid>,
    /// Has the user marked this Mark as "deleted"?
    pub deleted_at: Option<DateTime>,
    /// Has the user marked this Mark as "archived"?
    pub archived_at: Option<DateTime>,
}

#[derive(Debug, Copy, Clone, EnumIter, DeriveRelation)]
pub enum Relation {}

impl_timestamps!(ActiveModel);

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let mut ready = self;

        let data = ready.data.clone().unwrap();
        let mark: MarkData = serde_json::from_value(data).map_err(StoreError::from)?;
        ready.mark = Set(mark.type_name());
        ready.data = Set(serde_json::to_value(&mark).map_err(StoreError::from)?);

        ready.set_timestamps(insert);

        Ok(ready)
    }
}

impl TryFrom<Mark> for ActiveModel {
    type Error = StoreError;

    fn try_from(mark: Mark) -> StoreResult<Self> {
        Ok(ActiveModel {
            id: Unchanged(mark.id.inner()),
            mark: Set(mark.type_name()),
            data: Set(serde_json::to_value(mark.data)?),
            extra: Set(mark.extra),
            ..Default::default()
        })
    }
}

impl TryFrom<Model> for Mark {
    type Error = StoreError;

    fn try_from(model: Model) -> StoreResult<Self> {
        let mut builder = Mark::builder();

        builder = match model.mark.split_once(".") {
            Some(("content", _)) => {
                let data: Content = serde_json::from_value(model.data)?;
                builder.with_content(data)
            }
            Some(("landmark", _)) => {
                let data: Landmark = serde_json::from_value(model.data)?;
                builder.with_landmark(data)
            }
            _ => return Err(StoreError::UnknownMark(model.id.into(), model.mark)),
        };

        if let Some(extra) = model.extra {
            builder = builder.with_extra(extra);
        }

        let mark: Mark = builder.build();

        Ok(Self {
            id: model.id.into(),
            ..mark
        })
    }
}
