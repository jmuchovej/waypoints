//! Peers, as seen from here. Every Waypoints installation is a peer
//! (PROTOCOL.md §1); these are the shapes one takes when you talk to
//! it. Envelopes stay opaque throughout — typed conversion is the
//! caller's business, at its own boundary.

use std::future::Future;

use domain::{DeviceID, RealmID};
use serde_json::value::RawValue;

use crate::sync::peers::{
    PeerError, PeerResult,
    wire::{FetchResponse, UploadResponse},
};

/// A peer that stores and serves envelopes: the two §4 requests,
/// transport unspecified.
pub trait Peer: Send + Sync {
    /// Upload envelopes verbatim (§4.1). Idempotent on the peer's side;
    /// callers retry freely.
    fn upload(
        &self,
        realm_id: RealmID,
        envelopes: Vec<Box<RawValue>>,
    ) -> impl Future<Output = PeerResult<UploadResponse>> + Send;

    /// Fetch a page since `since_seq` (§4.2). `exclude_device` is the
    /// device echo filter (a bandwidth optimization only) — a pulling
    /// relay passes `None`: it wants everything.
    fn fetch_since(
        &self,
        realm_id: RealmID,
        since_seq: i64,
        exclude_device: Option<DeviceID>,
        limit: usize,
    ) -> impl Future<Output = PeerResult<FetchResponse>> + Send;
}

pub mod http;
#[cfg(feature = "testing")]
pub mod local;
#[cfg(feature = "testing")]
pub mod noop;

pub use http::HttpPeer;
#[cfg(feature = "testing")]
pub use local::LocalPeer;
#[cfg(feature = "testing")]
pub use noop::NoopPeer;
