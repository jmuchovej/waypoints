//! The network for Outpost: iroh transport,
//! length-prefixed postcard framing, Fleet-config-based connection auth,
//! and the session loop that drives the protocol's Tally/Resupply state
//! machines over a stream.
//!
//! Everything protocol-shaped stays transport-generic: the session loop
//! runs over any framed byte stream (a tokio duplex pipe in tests, an iroh
//! QUIC stream in production), and only the endpoint layer knows about iroh.
//! A `quinn`+static-address fallback would replace that layer alone.

pub(crate) mod endpoint;
pub mod fleet;
pub(crate) mod framing;
pub mod liveness;
pub(crate) mod session;
pub(crate) use outpost_protocol as protocol;

pub use endpoint::{ALPN, Endpoint, EndpointAddr, Session, TransportError};
pub use framing::{FrameError, MAX_FRAME, read_frame, write_frame};
pub use liveness::{HEARTBEAT_INTERVAL, IDLE_TIMEOUT, Liveness, is_keepalive, keepalive};
pub use session::{
    SessionError, SyncPeer, initiate, serve, serve_gated, serve_sync, serve_sync_gated,
};
