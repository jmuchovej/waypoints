use domain::DomainError;
use domain::chart::{ChartID, FeatureID, GlyphID};
use domain::mark::MarkID;
use thiserror::Error;

use crate::entities::MarkModel;

pub type StoreResult<T> = ::std::result::Result<T, StoreError>;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
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
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("unknown mark: {0}, [type={1}]")]
    UnknownMark(MarkID, String),
    #[error("{0}")]
    Custom(String),
}

impl From<StoreError> for sea_orm::DbErr {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Database(db_err) => db_err,
            _ => sea_orm::DbErr::Custom(err.to_string()),
        }
    }
}
