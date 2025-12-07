mod m20251118_203548_create_marks_table;
mod utils;

pub use sea_orm_migration::MigratorTrait;
use sea_orm_migration::{
    async_trait,
    prelude::{DynIden, IntoIden, MigrationTrait},
};

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migration_table_name() -> DynIden {
        "_migrations".into_iden()
    }

    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20251118_203548_create_marks_table::Migration)]
    }
}
