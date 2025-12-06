use derive_more::{Debug, From};
use sea_orm_migration::{
    DbErr,
    prelude::{Alias, IndexCreateStatement, IntoIden, TableRef},
    sea_orm::EntityTrait,
};
use thiserror::Error;
use waypoints_domain::DomainError;

#[allow(dead_code)]
pub type MigrationResult<T> = ::std::result::Result<T, MigrationError>;
#[allow(dead_code)]
pub type SeaDBResult<T> = ::std::result::Result<T, DbErr>;

#[derive(Debug, Error, From)]
pub enum MigrationError {
    #[error("Database error: {0}")]
    DatabaseError(DbErr),
    #[error("Domain error: {0}")]
    DomainError(DomainError),
    #[error("Chart not found: {0}")]
    ChartNotFound(String),
    #[error("JSON Error: {0}")]
    SerdeJSONError(serde_json::Error),
}

impl From<MigrationError> for sea_orm_migration::DbErr {
    fn from(err: MigrationError) -> Self {
        match err {
            MigrationError::DatabaseError(err) => err,
            _ => DbErr::Custom(err.to_string()),
        }
    }
}

#[allow(dead_code)]
pub fn table_name<E>(entity: E) -> SeaDBResult<TableRef>
where
    E: EntityTrait,
{
    Ok(TableRef::Table(Alias::new(entity.to_string()).into_iden()))
}

#[allow(dead_code)]
pub fn infer_index_name<E>(entity: E, i: IndexCreateStatement) -> SeaDBResult<String>
where
    E: EntityTrait,
{
    let column = i.get_index_spec().get_column_names().join("-");
    index_name(entity, &column)
}

#[allow(dead_code)]
pub fn index_name<E>(entity: E, c: &str) -> SeaDBResult<String>
where
    E: EntityTrait,
{
    let table = entity.to_string();
    let name = format!("idx-{table}-{c}");
    Ok(name)
}
