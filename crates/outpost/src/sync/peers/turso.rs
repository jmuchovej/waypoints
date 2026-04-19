//! The embedded turso/SQLite envelope store. `schema.sql` is the
//! canonical DDL, applied idempotently at boot — same SQL-is-truth
//! convention as the app's store crate.

use async_trait::async_trait;
use tokio::sync::Mutex;
use turso::{Connection, Value};

use super::store::{EnvelopeStore, GossipStore, StoreError, StoredEnvelope};
use super::wire::EnvelopeHead;

const SCHEMA: &str = include_str!("../../../schema.sql");

impl From<turso::Error> for StoreError {
    fn from(e: turso::Error) -> Self {
        StoreError(format!("database error: {e}"))
    }
}

pub struct Storage {
    // Relay traffic is tiny; one serialized connection is boring and
    // correct. Revisit if an outpost ever hosts busy realms.
    conn: Mutex<Connection>,
}

impl Storage {
    pub async fn open(path: &str) -> Result<Self, StoreError> {
        let db = turso::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        conn.pragma_update("journal_mode", "WAL").await?;
        conn.execute_batch(SCHEMA).await?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

#[async_trait]
impl EnvelopeStore for Storage {
    /// Insert-or-ignore each envelope, then read back every seq — fresh
    /// inserts and idempotent replays answer identically.
    async fn append(
        &self,
        heads: &[EnvelopeHead],
        payloads: &[&str],
        received_at: &str,
    ) -> Result<Vec<(String, i64)>, StoreError> {
        let conn = self.conn.lock().await;
        for (head, payload) in heads.iter().zip(payloads) {
            conn.execute(
                "INSERT INTO envelopes
                   (group_id, account_id, realm_id, device_id, hlc, payload, received_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT (group_id) DO NOTHING",
                (
                    head.group_id.as_str(),
                    head.account_id.as_str(),
                    head.realm_id.as_str(),
                    head.device_id.as_str(),
                    head.hlc_i64,
                    *payload,
                    received_at,
                ),
            )
            .await?;
        }

        let mut acks = Vec::with_capacity(heads.len());
        for head in heads {
            let mut rows = conn
                .query(
                    "SELECT seq FROM envelopes WHERE group_id = ?",
                    [head.group_id.as_str()],
                )
                .await?;
            let seq = match rows.next().await? {
                Some(row) => match row.get_value(0)? {
                    Value::Integer(n) => n,
                    _ => 0,
                },
                None => 0,
            };
            acks.push((head.group_id.clone(), seq));
        }
        Ok(acks)
    }

    /// Scan-window page: rows with `seq > since`, ascending, `limit + 1`
    /// to detect has_more. Device filtering happens in the caller so
    /// the window (and next_since) includes excluded envelopes.
    async fn page(
        &self,
        realm_id: &str,
        since_seq: i64,
        limit: usize,
    ) -> Result<Vec<StoredEnvelope>, StoreError> {
        let conn = self.conn.lock().await;
        let mut rows = conn
            .query(
                "SELECT seq, device_id, payload FROM envelopes
                 WHERE realm_id = ? AND seq > ?
                 ORDER BY seq ASC
                 LIMIT ?",
                (realm_id, since_seq, (limit + 1) as i64),
            )
            .await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let seq = match row.get_value(0)? {
                Value::Integer(n) => n,
                _ => continue,
            };
            let device_id = match row.get_value(1)? {
                Value::Text(s) => s,
                _ => continue,
            };
            let payload = match row.get_value(2)? {
                Value::Text(s) => s,
                _ => continue,
            };
            out.push(StoredEnvelope {
                seq,
                device_id,
                payload,
            });
        }
        Ok(out)
    }
}

#[async_trait]
impl GossipStore for Storage {
    async fn peer_cursor(&self, peer_id: &str, realm_id: &str) -> Result<i64, StoreError> {
        let conn = self.conn.lock().await;
        let mut rows = conn
            .query(
                "SELECT last_seq FROM peer_cursors WHERE peer_id = ? AND realm_id = ?",
                (peer_id, realm_id),
            )
            .await?;
        Ok(match rows.next().await? {
            Some(row) => match row.get_value(0)? {
                Value::Integer(n) => n,
                _ => 0,
            },
            None => 0,
        })
    }

    async fn set_peer_cursor(
        &self,
        peer_id: &str,
        realm_id: &str,
        last_seq: i64,
    ) -> Result<(), StoreError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO peer_cursors (peer_id, realm_id, last_seq)
             VALUES (?, ?, ?)
             ON CONFLICT (peer_id, realm_id) DO UPDATE SET last_seq = excluded.last_seq",
            (peer_id, realm_id, last_seq),
        )
        .await?;
        Ok(())
    }

    async fn known_realms(&self) -> Result<Vec<String>, StoreError> {
        let conn = self.conn.lock().await;
        let mut rows = conn
            .query("SELECT DISTINCT realm_id FROM envelopes", ())
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            if let Value::Text(s) = row.get_value(0)? {
                out.push(s);
            }
        }
        Ok(out)
    }
}
