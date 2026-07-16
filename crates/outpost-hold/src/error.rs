use outpost_protocol::WireError;

/// Failures in the storage layer. Bedrock and protocol wire errors surface
/// through here; storage never invents its own validation verdicts.
#[derive(Debug, thiserror::Error)]
pub enum HoldError {
    #[error(transparent)]
    Bedrock(#[from] bedrock::BedrockError),

    #[error(transparent)]
    Wire(#[from] WireError),

    /// A record already in the store failed to fold — the store is
    /// internally inconsistent, which is a bug, not a rejected input.
    #[error("stored ACL log does not fold: {0}")]
    AclFold(String),
}
