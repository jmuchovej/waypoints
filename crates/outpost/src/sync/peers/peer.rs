//! Peers, as seen from here. Every Waypoints installation is a peer
//! (PROTOCOL.md §1); these are the shapes one takes when you talk to
//! it. Envelopes stay opaque throughout — typed conversion is the
//! caller's business, at its own boundary.

use std::future::Future;

use domain::sync::{DeviceID, RealmID};
use serde::Serialize;
use serde_json::value::RawValue;

use super::service::{UploadRejection, accept_upload, serve_fetch};
use super::store::{EnvelopeStore, MemoryStore};
use super::wire::{FetchResponse, UploadResponse};

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("peer transport error: {0}")]
    Transport(String),
    #[error("peer rejected request ({status}): {message}")]
    Rejected { status: u16, message: String },
}

/// A peer that stores and serves envelopes: the two §4 requests,
/// transport unspecified.
pub trait Peer: Send + Sync {
    /// Upload envelopes verbatim (§4.1). Idempotent on the peer's side;
    /// callers retry freely.
    fn upload(
        &self,
        realm_id: RealmID,
        envelopes: Vec<Box<RawValue>>,
    ) -> impl Future<Output = Result<UploadResponse, PeerError>> + Send;

    /// Fetch a page since `since_seq` (§4.2). `exclude_device` is the
    /// device echo filter (a bandwidth optimization only) — a pulling
    /// relay passes `None`: it wants everything.
    fn fetch_since(
        &self,
        realm_id: RealmID,
        since_seq: i64,
        exclude_device: Option<DeviceID>,
        limit: usize,
    ) -> impl Future<Output = Result<FetchResponse, PeerError>> + Send;
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

    async fn parse<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, PeerError> {
        let status = response.status();
        if !status.is_success() {
            let message = response.text().await.unwrap_or_default();
            return Err(PeerError::Rejected {
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
    ) -> Result<UploadResponse, PeerError> {
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
    ) -> Result<FetchResponse, PeerError> {
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
    ) -> Result<UploadResponse, PeerError> {
        let refs: Vec<&RawValue> = envelopes.iter().map(AsRef::as_ref).collect();
        accept_upload(&self.store, realm_id, &refs, None)
            .await
            .map_err(|rejection| match rejection {
                UploadRejection::Malformed(message) => PeerError::Rejected {
                    status: 400,
                    message,
                },
                UploadRejection::RealmMismatch { group_id } => PeerError::Rejected {
                    status: 400,
                    message: format!("envelope {group_id} realm mismatch"),
                },
                UploadRejection::AccountMismatch { group_id } => PeerError::Rejected {
                    status: 403,
                    message: format!("envelope {group_id} account mismatch"),
                },
                UploadRejection::Store(e) => PeerError::Transport(e.to_string()),
            })
    }

    async fn fetch_since(
        &self,
        realm_id: RealmID,
        since_seq: i64,
        exclude_device: Option<DeviceID>,
        limit: usize,
    ) -> Result<FetchResponse, PeerError> {
        serve_fetch(&self.store, realm_id, since_seq, exclude_device, limit)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))
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
    ) -> Result<UploadResponse, PeerError> {
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
    ) -> Result<FetchResponse, PeerError> {
        Ok(FetchResponse {
            envelopes: Vec::new(),
            next_since: since_seq,
            has_more: false,
        })
    }
}
