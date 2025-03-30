pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table_tasks;
mod m20241226_012832_create_table_tasklists;
mod m20241226_163440_create_table_activities;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table_tasks::Migration),
            Box::new(m20241226_012832_create_table_tasklists::Migration),
            Box::new(m20241226_163440_create_table_activities::Migration),
        ]
    }

    fn migration_table_name() -> DynIden {
      Alias::new("seaql-migrations").into_iden()
    }
}
