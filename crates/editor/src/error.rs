use serde::Serialize;
use specta::{Generics, Type, TypeCollection, datatype as dtype};
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
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("store error: {0}")]
    Store(#[from] store::StoreError),
}

impl Serialize for EditorError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Type for EditorError {
    fn inline(_type_map: &mut TypeCollection, _generics: Generics) -> dtype::DataType {
        use dtype::{DataType, PrimitiveType};
        DataType::Primitive(PrimitiveType::String)
    }
}
