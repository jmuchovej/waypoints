//! The client half of §4 against a remote peer, envelopes kept opaque.
//! Typed envelope knowledge stays with the caller: cartograph converts
//! at the apply boundary, the gossip pump never converts at all.

use serde::Serialize;
use serde_json::value::RawValue;

use super::wire::{FetchResponse, UploadResponse};

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("peer transport error: {0}")]
    Transport(String),
    #[error("peer rejected request ({status}): {message}")]
    Rejected { status: u16, message: String },
}

#[derive(Debug, Clone)]
pub struct PeerClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl PeerClient {
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

    /// Fetch a page since `since_seq`. `exclude_device` is the device
    /// echo filter (a bandwidth optimization only, §4.2) — a pulling
    /// relay passes `None`: it wants everything.
    pub async fn fetch_since(
        &self,
        realm_id: &str,
        since_seq: i64,
        exclude_device: Option<&str>,
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

    /// Upload envelopes verbatim (§4.1). Idempotent on the relay side;
    /// callers retry freely.
    pub async fn upload(
        &self,
        realm_id: &str,
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

    fn envelopes_url(&self, realm_id: &str) -> String {
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
