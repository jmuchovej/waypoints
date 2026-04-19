//! Peers, as seen from here. Every Waypoints installation is a peer
//! (PROTOCOL.md §1); these are the shapes one takes when you talk to
//! it. Envelopes stay opaque throughout — typed conversion is the
//! caller's business, at its own boundary.

use std::future::Future;

use domain::{DeviceID, RealmID};
use serde::Serialize;
use serde_json::value::RawValue;

use super::service::{accept_upload, serve_fetch};
use super::store::{EnvelopeStore, MemoryStore};
use super::wire::{FetchResponse, UploadResponse};
use super::{PeerError, PeerResult};

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

// ── HttpPeer ────────────────────────────────────────────────────────────

/// A remote peer over HTTPS — an outpost, or anything else serving §4.
#[derive(Debug, Clone)]
pub struct HttpPeer {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl HttpPeer {
    /// `base_url` without a trailing slash, e.g. `https://relay.example.com`.
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.into(),
            http: reqwest::Client::new(),
        }
    }

    /// This peer's cursor identity: its base URL. A changed URL form
    /// resets cursors, which idempotent refetch makes safe.
    pub fn peer_id(&self) -> &str {
        &self.base_url
    }

    fn envelopes_url(&self, realm_id: RealmID) -> String {
        format!("{}/realms/{}/envelopes", self.base_url, realm_id)
    }

    async fn parse<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> PeerResult<T> {
        let status = response.status();
        if !status.is_success() {
            let message = response.text().await.unwrap_or_default();
            return Err(PeerError::RejectedRequest {
                status: status.as_u16(),
                message,
            });
        }
        response
            .json()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))
    }
}

impl Peer for HttpPeer {
    async fn upload(
        &self,
        realm_id: RealmID,
        envelopes: Vec<Box<RawValue>>,
    ) -> PeerResult<UploadResponse> {
        #[derive(Serialize)]
        struct UploadRequest {
            envelopes: Vec<Box<RawValue>>,
        }

        let response = self
            .http
            .post(self.envelopes_url(realm_id))
            .bearer_auth(&self.token)
            .json(&UploadRequest { envelopes })
            .send()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        Self::parse(response).await
    }

    async fn fetch_since(
        &self,
        realm_id: RealmID,
        since_seq: i64,
        exclude_device: Option<DeviceID>,
        limit: usize,
    ) -> PeerResult<FetchResponse> {
        let mut query = vec![
            ("since_seq", since_seq.to_string()),
            ("limit", limit.to_string()),
        ];
        if let Some(device) = exclude_device {
            query.push(("exclude_device", device.to_string()));
        }
        let response = self
            .http
            .get(self.envelopes_url(realm_id))
            .bearer_auth(&self.token)
            .query(&query)
            .send()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;
        Self::parse(response).await
    }
}

// ── LocalPeer ───────────────────────────────────────────────────────────

/// A peer answered from a local envelope store — no transport, no auth.
/// Over a `MemoryStore` it is the driver-test double; over the app's
/// own log it becomes the loopback for in-app serving (PROTOCOL.md §8).
/// Either way it runs the same §4 semantics as the HTTP server, via
/// `service` — never a re-implementation.
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

// ── NoopPeer ────────────────────────────────────────────────────────────

/// Offline mode: acknowledges nothing (the outbox stays pending for a
/// real peer later) and has nothing to serve.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopPeer;

impl Peer for NoopPeer {
    async fn upload(
        &self,
        _realm_id: RealmID,
        _envelopes: Vec<Box<RawValue>>,
    ) -> PeerResult<UploadResponse> {
        Ok(UploadResponse {
            results: Vec::new(),
            max_seq: 0,
        })
    }

    async fn fetch_since(
        &self,
        _realm_id: RealmID,
        since_seq: i64,
        _exclude_device: Option<DeviceID>,
        _limit: usize,
    ) -> PeerResult<FetchResponse> {
        Ok(FetchResponse {
            envelopes: Vec::new(),
            next_since: since_seq,
            has_more: false,
        })
    }
}
