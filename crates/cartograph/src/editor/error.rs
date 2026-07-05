use serde::Serialize;
use specta::{
    Type, Types,
    datatype::{DataType, Primitive},
};
use uuid::Uuid;

pub type EditorResult<T> = ::std::result::Result<T, EditorError>;

#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("mark not found: {0}")]
    NotFound(Uuid),
    #[error("not a landmark: {0}")]
    NotLandmark(Uuid),
    #[error("invalid content: {0}")]
    InvalidContent(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("no active territory")]
    NoActiveTerritory,
    #[error("store error: {0}")]
    Store(#[from] store::StoreError),
}

impl Serialize for EditorError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Type for EditorError {
    fn definition(_types: &mut Types) -> DataType {
        DataType::Primitive(Primitive::str)
    }
}
