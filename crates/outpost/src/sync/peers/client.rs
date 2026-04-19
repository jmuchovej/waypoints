//! The client half of §4 against a remote peer, envelopes kept opaque.
//! Fetch-only for now: gossip is pull-only (PROTOCOL.md §1), and the
//! app's typed upload path lives in cartograph until it migrates here.

use super::wire::FetchResponse;

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

    /// Fetch a page since `since_seq`. No `exclude_device`: a pulling
    /// relay wants everything.
    pub async fn fetch_since(
        &self,
        realm_id: &str,
        since_seq: i64,
        limit: usize,
    ) -> Result<FetchResponse, PeerError> {
        let url = format!("{}/realms/{}/envelopes", self.base_url, realm_id);
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .query(&[
                ("since_seq", since_seq.to_string()),
                ("limit", limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

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
