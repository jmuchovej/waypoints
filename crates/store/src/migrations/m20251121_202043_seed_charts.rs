use super::utils::SeaDBResult;
use crate::builtin;
use crate::entities::{ActiveChart, Charts};
use domain::chart::Chart;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{Set, Unchanged},
    EntityTrait,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn builtin_charts() -> Vec<Chart> {
    vec![
        builtin::page::chart(),
        builtin::objective::chart(),
        builtin::note::chart(),
        builtin::person::chart(),
        builtin::campaign::chart(),
        builtin::meeting::chart(),
        builtin::bookmark::chart(),
        builtin::journal::chart(),
        builtin::book::chart(),
    ]
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> SeaDBResult<()> {
        let db = manager.get_connection();

        for chart in builtin_charts() {
            let Chart {
                id,
                name,
                description,
                icon,
                features,
            } = chart;
            let mut active = ActiveChart {
                id: Set(id.0.clone()),
                name: Set(name),
                description: Set(description),
                icon: Set(icon.map(|i| serde_json::to_value(i).unwrap())),
                features: Set(serde_json::to_value(features).unwrap()),
            };

            let exists = Charts::find_by_id(&id.0).one(db).await?;

            if exists.is_some() {
                active.id = Unchanged(id.0);
                active.update(db).await?;
            } else {
                active.insert(db).await?;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> SeaDBResult<()> {
        let db = manager.get_connection();
        Charts::delete_many().exec(db).await?;
        Ok(())
    }
}
