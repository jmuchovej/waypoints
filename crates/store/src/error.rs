use domain::{ChartID, DomainError, FeatureID, GlyphID, MarkID, TerrainID};
use thiserror::Error;

pub type StoreResult<T> = ::std::result::Result<T, StoreError>;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("bedrock error: {0}")]
    Bedrock(#[from] bedrock::BedrockError),
    #[error("turso error: {0}")]
    Turso(#[from] turso::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("Mark not found: {0}")]
    MarkNotFound(MarkID),
    #[error("Chart not found: {0}")]
    ChartNotFound(ChartID),
    #[error("Glyph not found: {0}")]
    GlyphNotFound(GlyphID),
    #[error("Feature not found: {0}")]
    FeatureNotFound(FeatureID),
    #[error("Edge not found: {0}")]
    EdgeNotFound(TerrainID),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("unknown mark: {0}, [type={1}]")]
    UnknownMark(MarkID, String),
    #[error("{0}")]
    Custom(String),
}
