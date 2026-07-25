//! Boundary errors for codecs and signature checks. Validation verdicts
//! (accept / reject / pending) are a separate vocabulary arriving with
//! validation in slice 3 — a malformed byte string is not a *rejected* Notice,
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

/// Failures sealing or opening encrypted material. Deliberately
/// coarse: AEAD failures are indistinguishable by construction, and finer
/// reporting would only help an attacker.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// Authenticated decryption failed: wrong key, tampered ciphertext, or
    /// mismatched associated data.
    #[error("authenticated decryption failed")]
    Aead,

    /// Key material has the wrong shape (length or encoding).
    #[error("malformed key material")]
    BadKey,

    /// The sealed byte string is shorter than its fixed-size framing.
    #[error("sealed data is truncated")]
    Truncated,

    /// Encoding the AAD (envelope header) failed.
    #[error(transparent)]
    Wire(#[from] WireError),
}
