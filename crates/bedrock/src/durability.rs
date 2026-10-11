//! Durability state: the verified pragma pair, readback, and explicit flushing.
//!
//! Flushing is the *primitive* only. Deciding when to flush — idle timers,
//! crash sentinels, shutdown hooks — is runtime policy that belongs to the
//! caller.

use crate::BedrockError;
use turso::Connection;
use turso_core::SyncMode;

/// The engine version bedrock was built against: the `turso_core` release
/// resolved in the workspace lockfile, recorded at build time (see
/// `build.rs`). This is the identifier a [`DurabilityFault`] report carries —
/// the engine exposes no useful version at runtime, and the `turso` facade
/// pin is not the engine (the workspace clamps `turso_core` separately; the
/// lockfile is what records the version actually compiled in).
pub const ENGINE_VERSION: &str = env!("BEDROCK_ENGINE_VERSION");

/// The spelling `PRAGMA synchronous = …` accepts for each [`SyncMode`].
///
/// `SyncMode` is the engine's own type ([`turso_core::SyncMode`],
/// re-exported as [`bedrock::SyncMode`](crate::SyncMode)); it carries no
/// pragma spelling or `Display` of its own, and orphan rules keep us from
/// adding one, so the conversions live here as free functions.
pub(crate) fn sync_pragma_value(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::Off => "OFF",
        SyncMode::Normal => "NORMAL",
        SyncMode::Full => "FULL",
    }
}

/// Map the integer `PRAGMA synchronous` reads back. `None` for a value
/// outside the pinned engine's domain (a future engine could add levels).
fn sync_from_level(level: i64) -> Option<SyncMode> {
    match level {
        0 => Some(SyncMode::Off),
        1 => Some(SyncMode::Normal),
        2 => Some(SyncMode::Full),
        _ => None,
    }
}

/// Journal mode as `PRAGMA journal_mode` reports it.
///
/// bedrock's own type by necessity, not preference: the engine's
/// [`JournalMode`][JM] (`turso_core::storage::journal_mode::JournalMode`)
/// sits in a `pub(crate)` module and is reachable through no public path —
/// `use`ing it is E0603. What bedrock needs is only "WAL or not" plus the
/// reported spelling for fault reports.
///
/// [JM]: https://docs.rs/turso_core/latest/src/turso_core/storage/journal_mode.rs.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalMode {
    /// Write-ahead logging — the mode every file-backed territory requires.
    WAL,
    /// Any other mode the engine reported (`delete`, `memory`, `mvcc`, …).
    /// Kept as reported for fault reports; distinguishing rollback-journal
    /// flavors buys nothing, since none of them is acceptable here.
    Other(String),
}

impl std::fmt::Display for JournalMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WAL => f.write_str("wal"),
            Self::Other(mode) => f.write_str(mode),
        }
    }
}

/// Effective durability state read back from a live connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurabilityState {
    /// The journal mode the engine reported. A property of the database file.
    pub journal_mode: JournalMode,
    /// The synchronous level the engine reported. Per-connection state, read
    /// from the write connection — the one whose commits it governs.
    pub synchronous: SyncMode,
}

/// A durability pragma whose effective value does not match what was
/// requested: the structured fault behind
/// [`BedrockError::DurabilityMismatch`], retrievable from
/// [`Database::durability_fault`](crate::Database::durability_fault).
///
/// A faulted database opens read-only: reads are preserved (the notes are on
/// disk and readable), writes are refused carrying this fault. Resolution
/// beyond reopening — e.g. live-migrating to a fresh database — belongs to the
/// app layer; this fault carries what that layer needs to drive it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurabilityFault {
    /// The pragma that did not take effect (`journal_mode` or `synchronous`).
    pub pragma: &'static str,
    /// The value the configuration requested.
    pub requested: String,
    /// The value the engine reported reading it back.
    pub effective: String,
}

impl DurabilityFault {
    /// A pre-filled bug-report body: expected and effective values, the
    /// build-time engine version ([`ENGINE_VERSION`]), and the platform.
    pub fn report(&self) -> String {
        indoc::formatdoc! {"
              Durability pragma `{pragma}` did not take effect.
              requested: {requested}
              effective: {effective}
              engine: turso_core {engine}
              platform: {os}-{arch}
            ",
            pragma = self.pragma,
            requested = &self.requested,
            effective = &self.effective,
            engine = ENGINE_VERSION,
            os = std::env::consts::OS,
            arch = std::env::consts::ARCH,
        }
        .trim_end()
        .into()
    }
}

impl std::fmt::Display for DurabilityFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pragma {} requested {} but reads back {}",
            self.pragma, self.requested, self.effective
        )
    }
}

/// Read the effective journal mode and synchronous level back from `conn`.
///
/// This is the one readback path: [`Pool::open`](crate::pool::Pool) verifies
/// through it and [`Database::durability`](crate::Database::durability)
/// reports through it. A synchronous integer outside the known domain is
/// itself a durability mismatch — a level we cannot vouch for.
pub(crate) async fn read_state(conn: &Connection) -> Result<DurabilityState, BedrockError> {
    let journal = query_scalar(conn, "PRAGMA journal_mode").await?;
    let journal_mode = match journal {
        turso::Value::Text(mode) if mode.eq_ignore_ascii_case("wal") => JournalMode::WAL,
        turso::Value::Text(mode) => JournalMode::Other(mode),
        other => JournalMode::Other(format!("{other:?}")),
    };

    let level = query_scalar(conn, "PRAGMA synchronous").await?;
    let raw = level.as_integer().copied().unwrap_or(-1);
    let synchronous = sync_from_level(raw).ok_or_else(|| {
        BedrockError::DurabilityMismatch(DurabilityFault {
            pragma: "synchronous",
            requested: "a known level (OFF, NORMAL, or FULL)".into(),
            effective: format!("{level:?}"),
        })
    })?;

    Ok(DurabilityState {
        journal_mode,
        synchronous,
    })
}

/// Run the first row of a single-column pragma query.
async fn query_scalar(conn: &Connection, sql: &str) -> Result<turso::Value, BedrockError> {
    let mut rows = conn.query(sql, ()).await?;
    let value = match rows.next().await? {
        Some(row) => row.get_value(0)?,
        None => turso::Value::Null,
    };
    // Drain so the statement is finished before the connection is reused.
    while rows.next().await?.is_some() {}
    Ok(value)
}

/// How to flush durable state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlushMode {
    /// Write dirty pages out to the write-ahead log (`cacheflush`), without a
    /// checkpoint. Not a durability fsync, and not what makes a committed
    /// write durable — commits are already synced per the configured
    /// [`SyncMode`](crate::SyncMode).
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
