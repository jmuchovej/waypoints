use crate::core::{DecodeError, EncodeError, SpecError};

/// Anything that can go wrong at the engine boundary.
///
/// The leaf errors from `bedrock-core` stay precise and converge here
/// (alongside engine and JSON failures) via `#[from]`.
#[derive(Debug, thiserror::Error)]
pub enum BedrockError {
    /// A collection's schema did not hold together.
    #[error("invalid collection spec: {0}")]
    Spec(#[from] SpecError),

    /// An entity could not be projected into a row.
    #[error(transparent)]
    Encode(#[from] EncodeError),

    /// A row could not be reassembled into an entity.
    #[error(transparent)]
    Decode(#[from] DecodeError),

    /// A temporal value carried sub-millisecond precision, which the engine's
    /// `date`/`time`/`timestamp` columns would silently truncate.
    #[error("field {field:?} has sub-millisecond precision that the engine would truncate")]
    SubMillisecond { field: String },

    /// An update referenced a column the collection does not have.
    #[error("unknown column {column:?}")]
    UnknownColumn { column: String },

    /// An `$unset` targeted a non-nullable column, which cannot hold NULL.
    #[error("cannot unset non-nullable column {column:?}")]
    UnsetNonNullable { column: String },

    /// A schema change could not be applied automatically (it was destructive).
    #[error(transparent)]
    Migration(#[from] crate::migrate::MigrationError),

    /// The database on disk was stamped with a different `user_version` than the
    /// one the caller requires.
    #[error("incompatible database version: want {want}, found {got}")]
    IncompatibleVersion { want: i64, got: i64 },

    /// An operation was attempted on a database whose pool has been closed.
    #[error("database is closed")]
    Closed,

    /// The turso engine reported an error.
    #[error("engine error: {0}")]
    Engine(#[from] turso::Error),

    /// A document could not be (de)serialized to or from its stored JSON text.
    #[error("document json error: {0}")]
    Json(#[from] serde_json::Error),
}
