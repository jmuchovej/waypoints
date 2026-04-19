//! The storage seam: what a peer needs from its envelope store
//! (PROTOCOL.md §7). Two implementations are expected — the embedded
//! turso file (`store-turso` feature) for standalone outposts, and the
//! app's own `sync_log` for in-app serving (lives app-side; §8).
use domain::{DeviceID, OpID, RealmID};

use super::wire::EnvelopeFace;

/// Opaque storage failure. Callers log it and degrade; nothing
/// protocol-level rides on the variant.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct StoreError(pub String);

/// A stored envelope header row (payload carried separately, verbatim).
pub struct StoredEnvelope {
    pub seq: i64,
    pub device_id: DeviceID,
    pub payload: String,
}

#[async_trait::async_trait]
pub trait EnvelopeStore: Send + Sync {
    /// Insert-or-ignore each envelope, then report every (group_id, seq)
    /// — fresh inserts and idempotent replays answer identically.
    async fn append(
        &self,
        faces: &[EnvelopeFace],
        payloads: &[&str],
        received_at: &str,
    ) -> Result<Vec<(OpID, i64)>, StoreError>;

    /// Scan-window page: rows with `seq > since`, ascending, `limit + 1`
    /// so callers can detect has_more. Device filtering is the caller's
    /// job — the window (and next_since) must include excluded envelopes.
    async fn page(
        &self,
        realm_id: RealmID,
        since_seq: i64,
        limit: usize,
    ) -> Result<Vec<StoredEnvelope>, StoreError>;
}

/// What the gossip pump needs beyond serving: durable per-(peer, realm)
/// cursors and the set of realms worth pulling.
#[async_trait::async_trait]
pub trait GossipStore: EnvelopeStore {
    async fn peer_cursor(&self, peer_id: &str, realm_id: RealmID) -> Result<i64, StoreError>;

    async fn set_peer_cursor(
        &self,
        peer_id: &str,
        realm_id: RealmID,
        last_seq: i64,
    ) -> Result<(), StoreError>;

    /// Realms present in the local store. The pump pulls these plus any
    /// configured extras (the bootstrap case: a fresh mirror holds no
    /// realms yet).
    async fn known_realms(&self) -> Result<Vec<RealmID>, StoreError>;
}

/// The always-on peer's log in miniature: an in-memory `EnvelopeStore`
/// for driver tests and `LocalPeer` loopbacks. Same contract as the
/// turso store — arrival-ordered seqs, group_id idempotency.
#[derive(Debug, Default)]
pub struct MemoryStore {
    rows: std::sync::Mutex<Vec<MemoryRow>>,
}

#[derive(Debug)]
struct MemoryRow {
    seq: i64,
    group_id: OpID,
    realm_id: RealmID,
    device_id: DeviceID,
    payload: String,
}

#[async_trait::async_trait]
impl EnvelopeStore for MemoryStore {
    async fn append(
        &self,
        faces: &[EnvelopeFace],
        payloads: &[&str],
        _received_at: &str,
    ) -> Result<Vec<(OpID, i64)>, StoreError> {
        let mut rows = self.rows.lock().unwrap();
        let mut acks = Vec::with_capacity(faces.len());
        for (face, payload) in faces.iter().zip(payloads) {
            let seq = match rows.iter().find(|r| r.group_id == face.group_id) {
                Some(existing) => existing.seq,
                None => {
                    let seq = rows.len() as i64 + 1;
                    rows.push(MemoryRow {
                        seq,
                        group_id: face.group_id,
                        realm_id: face.realm_id,
                        device_id: face.device_id,
                        payload: (*payload).to_string(),
                    });
                    seq
                }
            };
            acks.push((face.group_id, seq));
        }
        Ok(acks)
    }

    async fn page(
        &self,
        realm_id: RealmID,
        since_seq: i64,
        limit: usize,
    ) -> Result<Vec<StoredEnvelope>, StoreError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .iter()
            .filter(|r| r.realm_id == realm_id && r.seq > since_seq)
            .take(limit + 1)
            .map(|r| StoredEnvelope {
                seq: r.seq,
                device_id: r.device_id,
                payload: r.payload.clone(),
            })
            .collect())
    }
}

/// RFC 3339 UTC, second precision — `received_at` is informational,
/// never protocol-load-bearing.
pub fn rfc3339_now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
