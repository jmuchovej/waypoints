//! Boundary errors for codecs and signature checks. Validation verdicts
//! (accept / reject / pending) are a separate vocabulary arriving with
//! §5.4 in slice 3 — a malformed byte string is not a *rejected* Notice,
//! it is not a Notice at all.

/// Failures turning bytes into protocol objects (or signing them).
#[derive(Debug, thiserror::Error)]
pub enum WireError {
    /// The envelope's leading version byte is one this build cannot parse.
    #[error("unsupported envelope version {0}")]
    UnsupportedVersion(u8),

    /// The byte string is empty or cut short.
    #[error("truncated envelope")]
    Truncated,

    /// Postcard (de)serialization failure.
    #[error("malformed encoding: {0}")]
    Encoding(#[from] postcard::Error),

    /// The signature does not verify over the raw bytes.
    #[error("invalid signature: {0}")]
    Signature(#[from] ed25519_dalek::SignatureError),
}
