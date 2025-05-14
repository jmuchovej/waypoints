use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(TaskList::Table)
                .if_not_exists()
                .col(pk_uuid(TaskList::UUID))
                .col(string(TaskList::Name))
                .col(string(TaskList::Description))
                .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager .drop_table(
            Table::drop().table(TaskList::Table).to_owned()
        ) .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
#[sea_orm(rename_all = "kebab-case")]
enum TaskList {
    Table,
    /// We need a unique identifier, this is it.
    UUID,
    /// What's the name of this `TaskList`?
    Name,
    /// What's this `TaskList` focused on?
    Description,
}
