//! The envelope store: one embedded turso/SQLite file. `schema.sql` is
//! the canonical DDL, applied idempotently at boot — same SQL-is-truth
//! convention as the app's store crate.

use tokio::sync::Mutex;
use turso::{Connection, Value};

const SCHEMA: &str = include_str!("../schema.sql");

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Db(#[from] turso::Error),
}

/// A stored envelope header row (payload carried separately, verbatim).
pub struct StoredEnvelope {
    pub seq: i64,
    pub device_id: String,
    pub payload: String,
}

pub struct Storage {
    // Relay traffic is tiny; one serialized connection is boring and
    // correct. Revisit if an outpost ever hosts busy realms.
    conn: Mutex<Connection>,
}

impl Storage {
    pub async fn open(path: &str) -> Result<Self, StorageError> {
        let db = turso::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        conn.pragma_update("journal_mode", "WAL").await?;
        conn.execute_batch(SCHEMA).await?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert-or-ignore each envelope, then read back every seq — fresh
    /// inserts and idempotent replays answer identically.
    pub async fn append(
        &self,
        envelopes: &[crate::handlers::EnvelopeHead],
        payloads: &[&str],
        received_at: &str,
    ) -> Result<Vec<(String, i64)>, StorageError> {
        let conn = self.conn.lock().await;
        for (head, payload) in envelopes.iter().zip(payloads) {
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

        let mut acks = Vec::with_capacity(envelopes.len());
        for head in envelopes {
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
    /// to detect has_more. Device filtering happens in the handler so
    /// the window (and next_since) includes excluded envelopes.
    pub async fn page(
        &self,
        realm_id: &str,
        since_seq: i64,
        limit: usize,
    ) -> Result<Vec<StoredEnvelope>, StorageError> {
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
