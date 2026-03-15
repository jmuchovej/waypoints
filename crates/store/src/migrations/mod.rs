mod m20251118_203548_create_marks_table;
mod m20251118_203608_create_charts_table;
mod m20251118_203617_create_terrains_table;
#[cfg(feature = "mock")]
mod m20251121_202202_seed_mock_data;
mod utils;

#[cfg(feature = "mock")]
pub(crate) mod mock {
    pub(crate) mod markdown;
    mod mock_charts;
    mod mock_marks;
    mod mock_terrains;

    use mock_charts::ChartIDMap;
    use mock_marks::MarkIDMap;

    pub(crate) use mock_charts::seed_charts;
    pub(crate) use mock_marks::seed_marks;
    pub(crate) use mock_terrains::seed_terrains;
}

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
        #[allow(unused_mut)]
        let mut migrations: Vec<Box<dyn MigrationTrait>> = vec![
            Box::new(m20251118_203548_create_marks_table::Migration),
            Box::new(m20251118_203608_create_charts_table::Migration),
            Box::new(m20251118_203617_create_terrains_table::Migration),
        ];

        #[cfg(feature = "mock")]
        migrations.push(Box::new(m20251121_202202_seed_mock_data::Migration));

        migrations
    }
}
