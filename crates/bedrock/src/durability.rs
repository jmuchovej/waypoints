//! Flushing durable state to disk: WAL checkpoints and page-cache fsync.
//!
//! This is the *primitive* only. Deciding when to flush — idle timers, crash
//! sentinels, shutdown hooks — is runtime policy that belongs to the caller.

use turso::Connection;

use crate::error::BedrockError;

/// How to flush durable state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlushMode {
    /// fsync the page cache without a WAL checkpoint.
    Fsync,
    /// `PRAGMA wal_checkpoint(PASSIVE)` — checkpoint what it can without
    /// blocking readers or writers (the default).
    #[default]
    Passive,
    /// `PRAGMA wal_checkpoint(FULL)` — checkpoint the whole WAL, waiting for
    /// readers to clear.
    Full,
    /// `PRAGMA wal_checkpoint(RESTART)` — like `Full`, and restart the WAL.
    Restart,
}

/// Flush the connection's durable state per `mode`.
pub(crate) async fn flush(conn: &Connection, mode: FlushMode) -> Result<(), BedrockError> {
    match mode {
        FlushMode::Fsync => {
            conn.cacheflush()?;
            Ok(())
        }
        FlushMode::Passive => checkpoint(conn, "PASSIVE").await,
        FlushMode::Full => checkpoint(conn, "FULL").await,
        FlushMode::Restart => checkpoint(conn, "RESTART").await,
    }
}

async fn checkpoint(conn: &Connection, mode: &str) -> Result<(), BedrockError> {
    let mut rows = conn
        .query(&format!("PRAGMA wal_checkpoint({mode})"), ())
        .await?;
    // Drain the checkpoint's (busy, log, checkpointed) result row.
    while rows.next().await?.is_some() {}
    Ok(())
}
