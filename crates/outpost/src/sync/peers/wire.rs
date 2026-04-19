//! The §4 wire contract, shared by the serving side (serialize) and the
//! client side (deserialize). Identity types are the frozen domain
//! newtypes — stable forever, so the log plane may speak them. The
//! `checkpoints` payload stays opaque throughout: a `Box<RawValue>` in,
//! the same bytes out.

use domain::sync::{AccountID, DeviceID, HLC, OpID, RealmID};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub const MAX_BATCH: usize = 100;
pub const MAX_BODY_BYTES: usize = 1 << 20; // 1 MiB
pub const DEFAULT_LIMIT: usize = 200;
pub const MAX_LIMIT: usize = 500;

/// The routing fields a relay reads. Everything else in an envelope —
/// `checkpoints` above all — passes through byte-verbatim. The app's
/// typed `Envelope` flattens this same struct, so the routing contract
/// exists exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeFace {
    /// Group-level idempotency key.
    pub group_id: OpID,
    pub account_id: AccountID,
    pub realm_id: RealmID,
    /// The originating device's real ID at origination time. Never
    /// rewritten, even when devices are aliased later — that would
    /// retroactively change LWW outcomes.
    pub device_id: DeviceID,
    #[serde(with = "hlc_wire")]
    pub hlc: HLC,
}

impl EnvelopeFace {
    /// Parse the routing fields out of a raw envelope. IDs must be
    /// UUIDs and the HLC a non-negative decimal string (PROTOCOL.md §2).
    pub fn parse(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|e| format!("malformed envelope: {e}"))
    }
}

/// §2 wire form of an HLC: a decimal *string*. Packed values exceed
/// `Number.MAX_SAFE_INTEGER`, so a bare JSON number would be silently
/// corrupted by any JavaScript participant that re-parses the envelope.
/// Shared by `EnvelopeFace` and the app's typed `Envelope`.
pub mod hlc_wire {
    use domain::sync::HLC;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(hlc: &HLC, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hlc.inner().to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<HLC, D::Error> {
        let raw = String::deserialize(d)?;
        let packed = raw.parse::<i64>().map_err(D::Error::custom)?;
        if packed < 0 {
            return Err(D::Error::custom("hlc must be non-negative"));
        }
        Ok(HLC(packed))
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UploadAck {
    pub group_id: OpID,
    pub seq: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub results: Vec<UploadAck>,
    pub max_seq: i64,
}
