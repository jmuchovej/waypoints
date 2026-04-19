use std::future::Future;

use domain::sync::{DeviceID, OpID, RealmID};
use outpost::sync::peers::client::{PeerClient, PeerError};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

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
/// HTTPS — a typed adapter over the outpost engine's `PeerClient`. The
/// wire carries raw envelopes; conversion to typed `Envelope`s happens
/// here, at the apply boundary, and nowhere below it.
#[derive(Debug, Clone)]
pub struct HttpRelayClient {
    peer: PeerClient,
}

impl HttpRelayClient {
    /// `base_url` without a trailing slash, e.g. `https://relay.example.com`.
    pub fn new(base_url: impl Into<String>, device_token: impl Into<String>) -> Self {
        Self {
            peer: PeerClient::new(base_url, device_token),
        }
    }
}

fn peer_error(e: PeerError) -> RelayError {
    match e {
        PeerError::Transport(message) => RelayError::Transport(message),
        PeerError::Rejected { status, message } => RelayError::Rejected { status, message },
    }
}

impl RelayClient for HttpRelayClient {
    async fn upload(
        &self,
        realm: RealmID,
        envelopes: Vec<Envelope>,
    ) -> Result<Vec<UploadAck>, RelayError> {
        let raw: Vec<Box<RawValue>> = envelopes
            .iter()
            .map(|envelope| serde_json::to_string(envelope).and_then(RawValue::from_string))
            .collect::<Result<_, _>>()
            .map_err(|e| RelayError::Transport(format!("envelope serialization: {e}")))?;
        let response = self
            .peer
            .upload(&realm.to_string(), raw)
            .await
            .map_err(peer_error)?;
        response
            .results
            .into_iter()
            .map(|ack| {
                let group_id = uuid::Uuid::parse_str(&ack.group_id).map_err(|e| {
                    RelayError::Transport(format!("relay returned malformed group_id: {e}"))
                })?;
                Ok(UploadAck {
                    group_id: group_id.into(),
                    seq: ack.seq,
                })
            })
            .collect()
    }

    async fn fetch_since(
        &self,
        realm: RealmID,
        since_seq: i64,
        exclude_device: DeviceID,
        limit: usize,
    ) -> Result<FetchPage, RelayError> {
        let page = self
            .peer
            .fetch_since(
                &realm.to_string(),
                since_seq,
                Some(&exclude_device.to_string()),
                limit,
            )
            .await
            .map_err(peer_error)?;
        let envelopes = page
            .envelopes
            .into_iter()
            .map(|sequenced| {
                let envelope: Envelope =
                    serde_json::from_str(sequenced.envelope.get()).map_err(|e| {
                        RelayError::Transport(format!("malformed envelope from relay: {e}"))
                    })?;
                Ok(SequencedEnvelope {
                    seq: sequenced.seq,
                    envelope,
                })
            })
            .collect::<Result<_, RelayError>>()?;
        Ok(FetchPage {
            envelopes,
            next_since: page.next_since,
            has_more: page.has_more,
        })
    }
}
