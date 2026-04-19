//! The §4 request semantics, transport-free: one implementation shared
//! by the HTTP handlers (`serve`, which adds auth and body caps) and
//! `LocalPeer` (loopback/tests). Protocol logic must not fork between
//! them — a test double that re-implements the protocol can drift from
//! it silently.

use domain::sync::{AccountID, DeviceID, RealmID};
use serde_json::value::RawValue;

use super::error::{PeerError, PeerResult};
use super::store::{EnvelopeStore, rfc3339_now};
use super::wire::{EnvelopeFace, FetchResponse, SequencedEnvelope, UploadAck, UploadResponse};

/// §4.1 minus transport concerns: parse faces, guard realm (and account,
/// when the caller's auth mode knows one), append idempotently, ack
/// every envelope with its seq. Rejections come back as the structured
/// `PeerError` variants; `serve` maps them onto HTTP statuses.
pub async fn accept_upload<S>(
    store: &S,
    realm_id: RealmID,
    envelopes: &[&RawValue],
    token_account: Option<AccountID>,
) -> PeerResult<UploadResponse>
where
    S: EnvelopeStore + ?Sized,
{
    let mut faces = Vec::with_capacity(envelopes.len());
    for raw in envelopes {
        let face = EnvelopeFace::parse(raw.get()).map_err(PeerError::Malformed)?;
        if face.realm_id != realm_id {
            return Err(PeerError::RealmMismatch {
                group_id: face.group_id,
            });
        }
        if let Some(account) = token_account
            && face.account_id != account
        {
            return Err(PeerError::AccountMismatch {
                group_id: face.group_id,
            });
        }
        faces.push(face);
    }

    let payloads: Vec<&str> = envelopes.iter().map(|r| r.get()).collect();
    let acks = store.append(&faces, &payloads, &rfc3339_now()).await?;

    let results: Vec<UploadAck> = acks
        .into_iter()
        .map(|(group_id, seq)| UploadAck { group_id, seq })
        .collect();
    let max_seq = results.iter().map(|r| r.seq).max().unwrap_or(0);
    Ok(UploadResponse { results, max_seq })
}

/// §4.2 minus transport concerns: scan-window page, `has_more` via the
/// store's limit+1 overshoot, `next_since` advanced past device-excluded
/// envelopes too.
pub async fn serve_fetch<S>(
    store: &S,
    realm_id: RealmID,
    since_seq: i64,
    exclude_device: Option<DeviceID>,
    limit: usize,
) -> PeerResult<FetchResponse>
where
    S: EnvelopeStore + ?Sized,
{
    let window = store.page(realm_id, since_seq, limit).await?;
    let has_more = window.len() > limit;
    let window = &window[..window.len().min(limit)];
    let next_since = window.last().map(|s| s.seq).unwrap_or(since_seq);

    let envelopes = window
        .iter()
        .filter(|row| Some(row.device_id) != exclude_device)
        .filter_map(|row| {
            let raw = RawValue::from_string(row.payload.clone()).ok()?;
            Some(SequencedEnvelope {
                seq: row.seq,
                envelope: raw,
            })
        })
        .collect();

    Ok(FetchResponse {
        envelopes,
        next_since,
        has_more,
    })
}
