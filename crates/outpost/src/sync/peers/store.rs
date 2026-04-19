//! The storage seam: what a peer needs from its envelope store
//! (PROTOCOL.md §7). Two implementations are expected — the embedded
//! turso file (`store-turso` feature) for standalone outposts, and the
//! app's own `sync_log` for in-app serving (lives app-side; §8).

use async_trait::async_trait;

use super::wire::EnvelopeHead;

/// Opaque storage failure. Callers log it and degrade; nothing
/// protocol-level rides on the variant.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct StoreError(pub String);

/// A stored envelope header row (payload carried separately, verbatim).
pub struct StoredEnvelope {
    pub seq: i64,
    pub device_id: String,
    pub payload: String,
}

#[async_trait]
pub trait EnvelopeStore: Send + Sync {
    /// Insert-or-ignore each envelope, then report every (group_id, seq)
    /// — fresh inserts and idempotent replays answer identically.
    async fn append(
        &self,
        heads: &[EnvelopeHead],
        payloads: &[&str],
        received_at: &str,
    ) -> Result<Vec<(String, i64)>, StoreError>;

    /// Scan-window page: rows with `seq > since`, ascending, `limit + 1`
    /// so callers can detect has_more. Device filtering is the caller's
    /// job — the window (and next_since) must include excluded envelopes.
    async fn page(
        &self,
        realm_id: &str,
        since_seq: i64,
        limit: usize,
    ) -> Result<Vec<StoredEnvelope>, StoreError>;
}

/// What the gossip pump needs beyond serving: durable per-(peer, realm)
/// cursors and the set of realms worth pulling.
#[async_trait]
pub trait GossipStore: EnvelopeStore {
    async fn peer_cursor(&self, peer_id: &str, realm_id: &str) -> Result<i64, StoreError>;

    async fn set_peer_cursor(
        &self,
        peer_id: &str,
        realm_id: &str,
        last_seq: i64,
    ) -> Result<(), StoreError>;

    /// Realms present in the local store. The pump pulls these plus any
    /// configured extras (the bootstrap case: a fresh mirror holds no
    /// realms yet).
    async fn known_realms(&self) -> Result<Vec<String>, StoreError>;
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
    group_id: String,
    realm_id: String,
    device_id: String,
    payload: String,
}

#[async_trait]
impl EnvelopeStore for MemoryStore {
    async fn append(
        &self,
        heads: &[EnvelopeHead],
        payloads: &[&str],
        _received_at: &str,
    ) -> Result<Vec<(String, i64)>, StoreError> {
        let mut rows = self.rows.lock().unwrap();
        let mut acks = Vec::with_capacity(heads.len());
        for (head, payload) in heads.iter().zip(payloads) {
            let seq = match rows.iter().find(|r| r.group_id == head.group_id) {
                Some(existing) => existing.seq,
                None => {
                    let seq = rows.len() as i64 + 1;
                    rows.push(MemoryRow {
                        seq,
                        group_id: head.group_id.clone(),
                        realm_id: head.realm_id.clone(),
                        device_id: head.device_id.clone(),
                        payload: (*payload).to_string(),
                    });
                    seq
                }
            };
            acks.push((head.group_id.clone(), seq));
        }
        Ok(acks)
    }

    async fn page(
        &self,
        realm_id: &str,
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
                device_id: r.device_id.clone(),
                payload: r.payload.clone(),
            })
            .collect())
    }
}

/// RFC 3339 UTC without pulling chrono into the dependency tree.
pub fn rfc3339_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // civil-from-days (Howard Hinnant's algorithm)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}
