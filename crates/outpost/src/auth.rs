//! Request authentication. Two modes, one seam (PROTOCOL.md §6):
//!
//! - `SharedSecret` — self-hosted outposts: one secret, no dependency on
//!   the global accounts database, no per-device identity.
//! - `AccountsDb` — the official outpost: per-device bearer tokens
//!   validated against the global Turso accounts database over Hrana.
//!   The accounts DB authenticates; it never stores sync data.

use serde_json::json;
use sha2::{Digest, Sha256};

/// Who a validated request is acting as. `SharedSecret` mode has no
/// per-device identity, so envelope account checks are skipped there.
#[derive(Debug, Clone, Default)]
pub struct Identity {
    pub account_id: Option<String>,
    pub device_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("missing bearer token")]
    Missing,
    #[error("unknown or revoked device token")]
    Unknown,
    #[error("account has no grant for this realm")]
    NoGrant,
    #[error("accounts database unreachable: {0}")]
    Upstream(String),
}

pub enum Validator {
    SharedSecret(String),
    AccountsDb {
        url: String,
        token: String,
        http: reqwest::Client,
    },
}

impl Validator {
    pub async fn validate(&self, bearer: &str, realm_id: &str) -> Result<Identity, AuthError> {
        if bearer.is_empty() {
            return Err(AuthError::Missing);
        }
        match self {
            Validator::SharedSecret(secret) => {
                // Length-safe comparison; secrets are operator-chosen.
                if constant_time_eq(bearer.as_bytes(), secret.as_bytes()) {
                    Ok(Identity::default())
                } else {
                    Err(AuthError::Unknown)
                }
            }
            Validator::AccountsDb { url, token, http } => {
                let hash = hex(Sha256::digest(bearer.as_bytes()).as_slice());
                let rows = hrana_query(
                    http,
                    url,
                    token,
                    "SELECT account_id, device_id FROM device_tokens
                     WHERE token_hash = ? AND revoked_at IS NULL",
                    &[&hash],
                )
                .await?;
                let Some(row) = rows.first() else {
                    return Err(AuthError::Unknown);
                };
                let identity = Identity {
                    account_id: row.first().cloned(),
                    device_id: row.get(1).cloned(),
                };

                let account = identity.account_id.clone().unwrap_or_default();
                let grants = hrana_query(
                    http,
                    url,
                    token,
                    "SELECT 1 FROM realm_grants WHERE realm_id = ? AND account_id = ?",
                    &[realm_id, &account],
                )
                .await?;
                if grants.is_empty() {
                    return Err(AuthError::NoGrant);
                }
                Ok(identity)
            }
        }
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Minimal Hrana-over-HTTP: one parameterized SELECT, text args, text
/// results. All the accounts DB contract needs.
async fn hrana_query(
    http: &reqwest::Client,
    url: &str,
    token: &str,
    sql: &str,
    args: &[&str],
) -> Result<Vec<Vec<String>>, AuthError> {
    let body = json!({
        "requests": [
            { "type": "execute", "stmt": {
                "sql": sql,
                "args": args.iter()
                    .map(|a| json!({ "type": "text", "value": a }))
                    .collect::<Vec<_>>(),
            }},
            { "type": "close" }
        ]
    });
    let response = http
        .post(format!("{}/v2/pipeline", url.trim_end_matches('/')))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| AuthError::Upstream(e.to_string()))?;
    if !response.status().is_success() {
        return Err(AuthError::Upstream(format!("status {}", response.status())));
    }
    let parsed: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AuthError::Upstream(e.to_string()))?;

    let rows = parsed["results"][0]["response"]["result"]["rows"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    Ok(rows
        .into_iter()
        .map(|row| {
            row.as_array()
                .map(|cells| {
                    cells
                        .iter()
                        .map(|c| c["value"].as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect())
}
