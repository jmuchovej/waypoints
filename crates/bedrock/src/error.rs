use bedrock_core::{DecodeError, EncodeError, SpecError};

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

    /// The turso engine reported an error.
    #[error("engine error: {0}")]
    Engine(#[from] turso::Error),

    /// A document could not be (de)serialized to or from its stored JSON text.
    #[error("document json error: {0}")]
    Json(#[from] serde_json::Error),
}
