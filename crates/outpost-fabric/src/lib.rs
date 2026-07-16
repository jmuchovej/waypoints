//! The network for Outpost (§10.3, design doc §12): iroh transport,
//! length-prefixed postcard framing, Fleet-config-based connection auth,
//! and the session loop that drives the protocol's Tally/Resupply state
//! machines over a stream.
//!
//! Everything protocol-shaped stays transport-generic: the [`session`]
//! runs over any framed byte stream (a tokio duplex pipe in tests, an iroh
//! QUIC stream in production), and only [`endpoint`] knows about iroh. A
//! `quinn`+static-address fallback (§10.3) would replace `endpoint` alone.

pub mod endpoint;
pub mod fleet;
pub mod framing;
pub mod session;

pub use endpoint::{ALPN, Endpoint, EndpointAddr, Session, TransportError};
pub use fleet::{FleetConfig, FleetError, FleetId, StationEntry, StationRoles};
pub use framing::{FrameError, MAX_FRAME, read_frame, write_frame};
pub use session::{
    SessionError, SyncPeer, initiate, serve, serve_gated, serve_sync, serve_sync_gated,
};
