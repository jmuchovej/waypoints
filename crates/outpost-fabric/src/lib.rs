// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of Waypoints.
//
// Waypoints is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Waypoints is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.
//
// Additional permission under GNU AGPL version 3 section 7: this Program
// may be conveyed through application stores on the terms set out in
// docs/licensing/COPYING.exceptions.

//! The network for Outpost: iroh transport,
//! length-prefixed postcard framing, Fleet-config-based connection auth,
//! and the session loop that drives the protocol's Tally/Resupply state
//! machines over a stream.
//!
//! Everything protocol-shaped stays transport-generic: the session loop
//! runs over any framed byte stream (a tokio duplex pipe in tests, an iroh
//! QUIC stream in production), and only the endpoint layer knows about iroh.
//! A `quinn`+static-address fallback would replace that layer alone.

// Features are additive and flavors are not: one shipped flavor per build.
#[cfg(all(feature = "nightly", feature = "preview"))]
compile_error!(
    "outpost-fabric: the `nightly` and `preview` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "nightly", feature = "stable"))]
compile_error!(
    "outpost-fabric: the `nightly` and `stable` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "preview", feature = "stable"))]
compile_error!(
    "outpost-fabric: the `preview` and `stable` flavors are both enabled; a build takes exactly one"
);

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
