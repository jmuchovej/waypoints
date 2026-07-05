use std::future::Future;

use domain::sync::{DeviceID, OpID, RealmID};
use serde::{Deserialize, Serialize};


use super::envelope::Envelope;

/// Transport-level failures talking to a relay. Protocol semantics —
/// idempotency, realm guarding, drift quarantine — are the coordinator's
/// job, not the client's.
#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error("relay transport error: {0}")]
    Transport(String),
    #[error("relay rejected request ({status}): {message}")]
    Rejected { status: u16, message: String },
}

/// The relay's acknowledgment of one envelope: the arrival seq it was
/// assigned, or already had (idempotent re-upload).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadAck {
    pub group_id: OpID,
    pub seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencedEnvelope {
    pub seq: i64,
    pub envelope: Envelope,
}

/// One page of the fetch scan. `next_since` is the highest seq *scanned*
/// (not returned) — it advances past excluded envelopes so a device that
/// only ever hears its own echoes still makes cursor progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchPage {
    pub envelopes: Vec<SequencedEnvelope>,
    pub next_since: i64,
    pub has_more: bool,
}

/// A peer that stores and serves envelopes. See relay/PROTOCOL.md — every
/// installation is conceptually one of these; the implementations here
/// are the client side of the always-online flavor.
pub trait RelayClient: Send + Sync {
    fn upload(
        &self,
        realm: RealmID,
        envelopes: Vec<Envelope>,
    ) -> impl Future<Output = Result<Vec<UploadAck>, RelayError>> + Send;

    fn fetch_since(
        &self,
        realm: RealmID,
        since_seq: i64,
        exclude_device: DeviceID,
        limit: usize,
    ) -> impl Future<Output = Result<FetchPage, RelayError>> + Send;
}

/// Offline mode: acknowledges nothing (the outbox stays pending for a
/// real relay later) and has nothing to serve.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopRelayClient;

impl RelayClient for NoopRelayClient {
    async fn upload(
        &self,
        _realm: RealmID,
        _envelopes: Vec<Envelope>,
    ) -> Result<Vec<UploadAck>, RelayError> {
        Ok(Vec::new())
    }

    async fn fetch_since(
        &self,
        _realm: RealmID,
        since_seq: i64,
        _exclude_device: DeviceID,
        _limit: usize,
    ) -> Result<FetchPage, RelayError> {
        Ok(FetchPage {
            envelopes: Vec::new(),
            next_since: since_seq,
            has_more: false,
        })
    }
}

/// The always-on peer in miniature, for tests: arrival-ordered seqs,
/// group_id idempotency, window-scan paging. Two `EditorService`s (or
/// coordinators) sharing one of these is a two-device realm.
#[derive(Debug, Default)]
pub struct InMemoryRelayClient {
    store: std::sync::Mutex<Vec<SequencedEnvelope>>,
}

impl RelayClient for InMemoryRelayClient {
    async fn upload(
        &self,
        _realm: RealmID,
        envelopes: Vec<Envelope>,
    ) -> Result<Vec<UploadAck>, RelayError> {
        let mut store = self.store.lock().unwrap();
        let acks = envelopes
            .into_iter()
            .map(|envelope| {
                let group_id = envelope.group_id;
                let seq = match store.iter().find(|s| s.envelope.group_id == group_id) {
                    Some(existing) => existing.seq,
                    None => {
                        let seq = store.len() as i64 + 1;
                        store.push(SequencedEnvelope { seq, envelope });
                        seq
                    }
                };
                UploadAck { group_id, seq }
            })
            .collect();
        Ok(acks)
    }

    async fn fetch_since(
        &self,
        realm: RealmID,
        since_seq: i64,
        exclude_device: DeviceID,
        limit: usize,
    ) -> Result<FetchPage, RelayError> {
        let store = self.store.lock().unwrap();
        // Scan the window first, then filter: next_since must advance
        // past excluded envelopes too.
        let window: Vec<&SequencedEnvelope> = store
            .iter()
            .filter(|s| s.seq > since_seq && s.envelope.realm_id == realm)
            .take(limit)
            .collect();
        let next_since = window.last().map(|s| s.seq).unwrap_or(since_seq);
        let has_more = store
            .iter()
            .any(|s| s.seq > next_since && s.envelope.realm_id == realm);
        let envelopes = window
            .into_iter()
            .filter(|s| s.envelope.device_id != exclude_device)
            .cloned()
            .collect();
        Ok(FetchPage {
            envelopes,
            next_since,
            has_more,
        })
    }
}

/// The client side of an always-on relay speaking relay/PROTOCOL.md over
/// HTTPS. Endpoint shapes are tested; integration against a live relay
/// is deferred (guide Phase 6).
#[derive(Debug, Clone)]
pub struct HttpRelayClient {
    base_url: String,
    device_token: String,
    http: reqwest::Client,
}

impl HttpRelayClient {
    /// `base_url` without a trailing slash, e.g. `https://relay.example.com`.
    pub fn new(base_url: impl Into<String>, device_token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            device_token: device_token.into(),
            http: reqwest::Client::new(),
        }
    }

    fn envelopes_url(&self, realm: RealmID) -> String {
        format!("{}/realms/{}/envelopes", self.base_url, realm)
    }

    async fn check(response: reqwest::Response) -> Result<reqwest::Response, RelayError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let message = response.text().await.unwrap_or_default();
        Err(RelayError::Rejected {
            status: status.as_u16(),
            message,
        })
    }
}

#[derive(Serialize)]
struct UploadRequest {
    envelopes: Vec<Envelope>,
}

#[derive(Deserialize)]
struct UploadResponse {
    results: Vec<UploadAck>,
}

impl RelayClient for HttpRelayClient {
    async fn upload(
        &self,
        realm: RealmID,
        envelopes: Vec<Envelope>,
    ) -> Result<Vec<UploadAck>, RelayError> {
        let response = self
            .http
            .post(self.envelopes_url(realm))
            .bearer_auth(&self.device_token)
            .json(&UploadRequest { envelopes })
            .send()
            .await
            .map_err(|e| RelayError::Transport(e.to_string()))?;
        let body: UploadResponse = Self::check(response)
            .await?
            .json()
            .await
            .map_err(|e| RelayError::Transport(e.to_string()))?;
        Ok(body.results)
    }

    async fn fetch_since(
        &self,
        realm: RealmID,
        since_seq: i64,
        exclude_device: DeviceID,
        limit: usize,
    ) -> Result<FetchPage, RelayError> {
        let response = self
            .http
            .get(self.envelopes_url(realm))
            .bearer_auth(&self.device_token)
            .query(&[
                ("since_seq", since_seq.to_string()),
                ("exclude_device", exclude_device.to_string()),
                ("limit", limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| RelayError::Transport(e.to_string()))?;
        Self::check(response)
            .await?
            .json()
            .await
            .map_err(|e| RelayError::Transport(e.to_string()))
    }
}
