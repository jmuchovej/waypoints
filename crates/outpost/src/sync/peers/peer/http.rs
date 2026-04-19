use domain::{DeviceID, RealmID};
use serde::Serialize;
use serde_json::value::RawValue;

use crate::sync::peers::{
    Peer, PeerError, PeerResult,
    wire::{FetchResponse, UploadResponse},
};

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
