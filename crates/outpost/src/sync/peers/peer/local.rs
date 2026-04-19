use domain::{DeviceID, RealmID};
use serde_json::value::RawValue;

use crate::sync::peers::{
    Peer, PeerResult,
    service::{accept_upload, serve_fetch},
    store::{EnvelopeStore, MemoryStore},
    wire::{FetchResponse, UploadResponse},
};

/// A peer answered from a local envelope store — no transport, no auth.
/// A **test fixture**: it exists so driver tests exercise the real §4
/// semantics via `service` (never a re-implementation), which is what
/// keeps them from drifting off the served protocol. It is *not* the
/// in-app serving path — serving consumes `service`/`serve` directly
/// over the app's store; a device pulling from its own log would only
/// re-fetch what idempotency then skips.
#[derive(Debug, Default)]
pub struct LocalPeer<S> {
    store: S,
}

impl<S: EnvelopeStore> LocalPeer<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

impl LocalPeer<MemoryStore> {
    /// The always-on peer in miniature: arrival-ordered seqs, group_id
    /// idempotency, window-scan paging. Two coordinators sharing one of
    /// these is a two-device realm.
    pub fn in_memory() -> Self {
        Self::new(MemoryStore::default())
    }
}

impl<S: EnvelopeStore> Peer for LocalPeer<S> {
    async fn upload(
        &self,
        realm_id: RealmID,
        envelopes: Vec<Box<RawValue>>,
    ) -> PeerResult<UploadResponse> {
        let refs: Vec<&RawValue> = envelopes.iter().map(AsRef::as_ref).collect();
        // Pure passthrough: the service already speaks PeerError, and a
        // local caller gets the structured rejection variants a remote
        // caller loses to the wire.
        accept_upload(&self.store, realm_id, &refs, None).await
    }

    async fn fetch_since(
        &self,
        realm_id: RealmID,
        since_seq: i64,
        exclude_device: Option<DeviceID>,
        limit: usize,
    ) -> PeerResult<FetchResponse> {
        serve_fetch(&self.store, realm_id, since_seq, exclude_device, limit).await
    }
}
