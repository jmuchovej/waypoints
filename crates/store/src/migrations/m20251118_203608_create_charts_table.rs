use super::utils::{SeaDBResult, infer_index_name, table_name};
use crate::entities::Charts as Entity;
use sea_orm_migration::{prelude::*, sea_orm::Schema};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> SeaDBResult<()> {
        // Replace the sample below with your own migration scripts
        let builder = m.get_database_backend();
        let schema = Schema::new(builder);

        let table = schema.create_table_from_entity(Entity);
        m.create_table(table).await?;

        let indexes = schema.create_index_from_entity(Entity);
        for index in indexes {
            m.create_index(index).await?;
        }

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> SeaDBResult<()> {
        // Replace the sample below with your own migration scripts
        let builder = m.get_database_backend();
        let schema = Schema::new(builder);

        let name = table_name(Entity)?.to_owned();

        let drop = Index::drop().table(name.clone()).to_owned();
        let indexes = schema.create_index_from_entity(Entity);
        for index in indexes {
            let index = infer_index_name(Entity, index)?;
            m.drop_index(drop.clone().name(index).to_owned()).await?;
        }

        let table = Table::drop().table(name).to_owned();
        m.drop_table(table).await?;

        Ok(())
    }
}
