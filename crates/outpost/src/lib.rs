//! Outpost — the always-online peer. See PROTOCOL.md in this crate.
//!
//! Deliberately dumb: stores envelope payloads byte-verbatim, assigns
//! per-realm arrival seqs, and never parses `checkpoints`. Conflict
//! resolution lives exclusively in the app's Rust core; an outpost that
//! tried to be clever would be wrong.
//!
//! Feature-delimited (an embedding app pulls in only what it needs):
//! - `peers` (default): wire types, fetch client, `EnvelopeStore` seam,
//!   gossip pump.
//! - `peers-serve`: the §4 endpoints as an axum router, plus auth.
//! - `store-turso`: the embedded turso envelope store.
//! - `binary`: everything the `outpost` binary needs.

/// The sync engine, feature-delimited (PROTOCOL.md §1). `peers` is the
/// store-and-forward peer engine — the only component that exists.
/// `relays` (live forwarding between simultaneously-online peers) and
/// `directory` (peer discovery) are reserved names for future
/// components; they gain modules when they gain code.
#[cfg(feature = "peers")]
pub mod sync {
    /// The store-and-forward peer engine: wire contract, fetch client,
    /// storage seam, and the gossip pump. Log-plane only — nothing in this
    /// tree may parse `checkpoints` or import maneuver types; a peer that
    /// stores envelopes it cannot parse is what survives app-version skew.
    pub mod peers {
        pub mod error;
        pub mod peer;
        pub mod pump;
        pub mod service;
        pub mod store;
        pub mod wire;

        #[cfg(feature = "peers-serve")]
        pub mod serve;
        #[cfg(feature = "store-turso")]
        pub mod turso;

        pub use error::{PeerError, PeerResult};
        pub use peer::{HttpPeer, LocalPeer, NoopPeer, Peer};
    }
}

#[cfg(feature = "peers-serve")]
pub mod auth;

#[cfg(feature = "peers-serve")]
pub use sync::peers::serve::{AppState, router};
