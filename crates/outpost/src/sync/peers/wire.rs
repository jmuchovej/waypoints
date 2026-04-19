//! The §4 wire contract, shared by the serving side (serialize) and the
//! client side (deserialize). Envelopes stay opaque throughout: a
//! `Box<RawValue>` in, the same bytes out.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub const MAX_BATCH: usize = 100;
pub const MAX_BODY_BYTES: usize = 1 << 20; // 1 MiB
pub const DEFAULT_LIMIT: usize = 200;
pub const MAX_LIMIT: usize = 500;

/// The routing fields a relay reads. Everything else in an envelope —
/// `checkpoints` above all — passes through byte-verbatim.
#[derive(Debug, Deserialize)]
pub struct EnvelopeHead {
    pub group_id: String,
    pub account_id: String,
    pub realm_id: String,
    pub device_id: String,
    pub hlc: String,
    #[serde(skip)]
    pub hlc_i64: i64,
}

impl EnvelopeHead {
    /// Parse the head fields out of a raw envelope, validating the HLC
    /// (a decimal string; PROTOCOL.md §2).
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut head: Self =
            serde_json::from_str(raw).map_err(|e| format!("malformed envelope: {e}"))?;
        head.hlc_i64 = match head.hlc.parse::<i64>() {
            Ok(v) if v >= 0 => v,
            _ => {
                return Err(format!(
                    "envelope {} hlc must be a decimal string",
                    head.group_id
                ));
            }
        };
        Ok(head)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SequencedEnvelope {
    pub seq: i64,
    pub envelope: Box<RawValue>,
}

/// One page of the §4.2 fetch scan. `next_since` is the highest seq
/// *scanned* (not returned) — it advances past device-excluded envelopes
/// too.
#[derive(Debug, Serialize, Deserialize)]
pub struct FetchResponse {
    pub envelopes: Vec<SequencedEnvelope>,
    pub next_since: i64,
    pub has_more: bool,
}

/// The relay's acknowledgment of one envelope: the arrival seq it was
/// assigned, or already had (idempotent re-upload).
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadAck {
    pub group_id: String,
    pub seq: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub results: Vec<UploadAck>,
    pub max_seq: i64,
}
