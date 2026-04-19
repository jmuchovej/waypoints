//! Every way the peer engine fails, in one type. `PeerError` is the
//! single vocabulary for talking to, serving as, or storing for a peer;
//! seams that cannot produce a variant simply never construct it.
//! (Request auth is deliberately not here — `AuthError` belongs to the
//! serving transport, not the engine.)

use domain::sync::OpID;

pub type PeerResult<T> = std::result::Result<T, PeerError>;

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    /// Could not reach the peer at all.
    #[error("peer transport error: {0}")]
    Transport(String),

    /// A remote peer refused the request. This is the wire's flattened
    /// form of the structured rejections below — HTTP hands back only a
    /// status and a message, so `HttpPeer` cannot up-convert. A local
    /// peer reports the structured variant instead.
    #[error("peer rejected request ({status}): {message}")]
    RejectedRequest { status: u16, message: String },

    /// The local envelope store failed. Opaque: callers log and degrade,
    /// nothing protocol-level rides on the contents.
    #[error("storage failure: {0}")]
    Storage(String),

    // ── §4.1 rejections, structured ─────────────────────────────────
    // Produced where the refusing peer is local (`service`, and
    // `LocalPeer` by passthrough); `serve` maps them onto HTTP statuses
    // at the edge, which is where they collapse into RejectedRequest
    // for the caller on the other side of the wire.
    #[error("{0}")]
    Malformed(String),

    #[error("envelope {group_id} realm mismatch")]
    RealmMismatch { group_id: OpID },

    #[error("envelope {group_id} account mismatch")]
    AccountMismatch { group_id: OpID },
}
