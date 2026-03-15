use serde::Serialize;
use uuid::Uuid;

use specta::{Generics, Type, TypeCollection, datatype as dtype};

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
}

impl Serialize for MarkError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Type for MarkError {
    fn inline(_type_map: &mut TypeCollection, _generics: Generics) -> dtype::DataType {
        use dtype::{DataType, PrimitiveType};
        DataType::Primitive(PrimitiveType::String)
    }
}
