use thiserror::Error;

pub type DomainResult<T> = ::std::result::Result<T, DomainError>;
pub type DomainResults<T> = ::std::result::Result<T, Vec<DomainError>>;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Base36IDs only should be {expected:?} characters! Got {found:?} characters.")]
    Base36IDLengthMismatch { expected: usize, found: usize },
    #[error("Base36IDs must be alphanumeric characters.")]
    Base36IDInvalidCharacters,
    #[error("Failed to parse icon: {0}")]
    ParseIconFailed(String),
    #[error("Unknown domain error!")]
    Unknown,
}
