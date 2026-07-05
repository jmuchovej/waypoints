use serde::Serialize;
use uuid::Uuid;

use specta::{
    Type, Types,
    datatype::{DataType, Primitive},
};

pub type MarkResult<T> = ::std::result::Result<T, MarkError>;

#[derive(Debug, thiserror::Error)]
pub enum MarkError {
    #[error("no active territory")]
    NoActiveTerritory,
    #[error("mark not found: {0}")]
    NotFound(Uuid),
    #[error("not a landmark: {0}")]
    NotLandmark(Uuid),
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("store error: {0}")]
    Store(#[from] store::StoreError),
}

impl Serialize for MarkError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Type for MarkError {
    fn definition(_types: &mut Types) -> DataType {
        DataType::Primitive(Primitive::str)
    }
}
