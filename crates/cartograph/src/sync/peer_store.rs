//! The app's own log, served as an envelope store (PROTOCOL.md §8):
//! `sync_log` implementing outpost's `EnvelopeStore`, so the embedded
//! peer answers §4 from the same table every mutation already writes.
//!
//! The write path is log-plane on purpose: `append` stores face fields
//! plus the byte-verbatim payload and never parses `checkpoints` — a
//! carrier row. That is what lets this device store envelopes from app
//! versions it cannot parse. The domain plane catches up separately:
//! `SyncCoordinator::apply_pending` parses and applies carrier rows,
//! and leaves pending whatever it only knows how to carry.

use async_trait::async_trait;
use domain::sync::{OpID, RealmID};
use outpost::sync::peers::store::{EnvelopeStore, StoredEnvelope};
use outpost::sync::peers::wire::EnvelopeFace;
use outpost::sync::peers::{PeerError, PeerResult};
use store::DatabaseConnection;
use store::query::sync_log;

use super::driver::envelope_from_row;

pub struct AppEnvelopeStore {
    db: DatabaseConnection,
}

impl AppEnvelopeStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn storage(e: impl std::fmt::Display) -> PeerError {
    PeerError::Storage(e.to_string())
}

#[async_trait]
impl EnvelopeStore for AppEnvelopeStore {
    async fn append(
        &self,
        faces: &[EnvelopeFace],
        payloads: &[&str],
        _received_at: &str,
    ) -> PeerResult<Vec<(OpID, i64)>> {
        let mut acks = Vec::with_capacity(faces.len());
        for (face, payload) in faces.iter().zip(payloads) {
            let seq = sync_log::append_envelope(
                &self.db,
                face.group_id,
                face.device_id,
                face.account_id,
                face.realm_id,
                face.hlc,
                payload,
            )
            .await
            .map_err(storage)?;
            acks.push((face.group_id, seq));
        }
        Ok(acks)
    }

    async fn page(
        &self,
        realm_id: RealmID,
        since_seq: i64,
        limit: usize,
    ) -> PeerResult<Vec<StoredEnvelope>> {
        let rows = sync_log::page_envelopes(&self.db, realm_id, since_seq, (limit + 1) as u64)
            .await
            .map_err(storage)?;
        rows.into_iter()
            .map(|row| {
                let seq = row.local_seq;
                let device_id = row.device_id.into();
                // Rows predating the payload column (V0008) are rebuilt
                // from their parsed ops; everything newer serves the
                // stored bytes verbatim.
                let payload = match &row.payload {
                    Some(payload) => payload.clone(),
                    None => serde_json::to_string(&envelope_from_row(row).map_err(storage)?)
                        .map_err(storage)?,
                };
                Ok(StoredEnvelope {
                    seq,
                    device_id,
                    payload,
                })
            })
            .collect()
    }
}
