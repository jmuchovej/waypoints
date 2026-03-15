use thiserror::Error;
use waypoints_domain::mark::MarkID;

pub type StoreResult<T> = ::std::result::Result<T, StoreError>;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("unknown mark: {0}, [type={1}]")]
    UnknownMark(MarkID, String),
}

impl From<StoreError> for sea_orm::DbErr {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Database(db_err) => db_err,
            _ => sea_orm::DbErr::Custom(err.to_string()),
        }
    }
}
