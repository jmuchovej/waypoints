use domain::{DeviceID, RealmID};
use serde_json::value::RawValue;

use crate::sync::peers::{
    Peer, PeerResult,
    wire::{FetchResponse, UploadResponse},
};

/// A **test fixture** for the driver's defensive contract: a peer that
/// acks nothing must leave the outbox pending and must not spin the
/// upload loop (a protocol-violating relay could behave this way).
/// Production offline mode is simply not scheduling sync — not syncing
/// into a black hole.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopPeer;

impl Peer for NoopPeer {
    async fn upload(
        &self,
        _realm_id: RealmID,
        _envelopes: Vec<Box<RawValue>>,
    ) -> PeerResult<UploadResponse> {
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
    ) -> PeerResult<FetchResponse> {
        Ok(FetchResponse {
            envelopes: Vec::new(),
            next_since: since_seq,
            has_more: false,
        })
    }
}
