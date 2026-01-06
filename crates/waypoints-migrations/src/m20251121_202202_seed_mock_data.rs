use crate::mock::{seed_charts, seed_marks, seed_terrains};
use crate::utils::SeaDBResult;
use sea_orm::EntityTrait;
use sea_orm_migration::prelude::*;
use waypoints_entities::prelude::{Charts, Marks, Terrains};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> SeaDBResult<()> {
        let chart_ids = seed_charts(manager).await?;
        let mark_ids = seed_marks(manager, &chart_ids).await?;
        let _terrain_ids = seed_terrains(manager, &mark_ids, &chart_ids).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> SeaDBResult<()> {
        let db = manager.get_connection();

        // Delete in reverse dependency order
        Terrains::delete_many().exec(db).await?;
        Marks::delete_many().exec(db).await?;
        Charts::delete_many().exec(db).await?;

        Ok(())
    }
}
