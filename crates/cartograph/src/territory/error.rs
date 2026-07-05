use serde::Serialize;
use uuid::Uuid;

pub type TerritoryResult<T> = ::std::result::Result<T, TerritoryError>;

#[derive(Debug, thiserror::Error)]
pub enum TerritoryError {
    #[error("territory not found: {0}")]
    NotFound(Uuid),
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("store error: {0}")]
    Store(#[from] store::StoreError),
    #[error("config error: {0}")]
    Config(String),
}

impl Serialize for TerritoryError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl specta::Type for TerritoryError {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::str)
    }
}
