//! A 1-writer / N-reader connection pool over a single turso engine.
//!
//! turso permits only one in-flight operation per connection, so throughput
//! comes from having several: one serialized write connection (guarded by an
//! async mutex) and a lazily-grown set of read connections bounded by a
//! semaphore. All connections come from the same engine handle and therefore
//! see the same data — including for `:memory:`, whose database lives in the
//! shared engine rather than per-connection.
//!
//! Rust has no async `Drop`, so a connection returned mid-transaction (a
//! dropped, uncommitted [`WriteTx`](crate::WriteTx)) is cleaned with a
//! `ROLLBACK` on its *next* acquisition rather than on release.

use crate::{
    BedrockError, Config, SyncMode,
    durability::{self, DurabilityFault, JournalMode},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::{Mutex, OwnedMutexGuard, OwnedSemaphorePermit, Semaphore};
use turso::Connection;

/// The connection pool backing a [`Database`](crate::Database).
pub(crate) struct Pool {
    engine: turso::Database,
    write: Arc<Mutex<Option<Connection>>>,
    read_sem: Arc<Semaphore>,
    read_pool: Arc<std::sync::Mutex<Vec<Connection>>>,
    reads_created: AtomicUsize,
    read_limit: usize,
    pragma: Vec<(String, String)>,
    wal_for_files: bool,
    synchronous: SyncMode,
    is_memory: bool,
    closed: AtomicBool,
    /// Set once at open when a durability pragma did not take effect. A
    /// faulted pool is read-only: [`write`](Pool::write) refuses, reads are
    /// untouched.
    fault: Option<DurabilityFault>,
}

impl Pool {
    /// Open the write connection, applying pragmas and enforcing
    /// `config.version` against `PRAGMA user_version`.
    ///
    /// The durability pair (journal mode, synchronous level) is read back from
    /// the write connection and verified against the config. A mismatch does
    /// **not** fail the open — denying access to readable notes is worse than
    /// the degradation — it marks the pool faulted, and faulted pools refuse
    /// writes.
    ///
    /// `is_new` must be computed by the caller *before* the engine is built:
    /// building creates the database file, so an existence check afterwards
    /// would always report a pre-existing file.
    pub(crate) async fn open(
        path: &str,
        engine: turso::Database,
        config: Config,
        is_new: bool,
    ) -> Result<Pool, BedrockError> {
        let is_memory = path == ":memory:";
        let pragma: Vec<(String, String)> = config.pragma.into_iter().collect();

        let mut pool = Pool {
            engine,
            write: Arc::new(Mutex::new(None)),
            read_sem: Arc::new(Semaphore::new(config.read_connections)),
            read_pool: Arc::new(std::sync::Mutex::new(Vec::new())),
            reads_created: AtomicUsize::new(0),
            read_limit: config.read_connections,
            pragma,
            wal_for_files: config.wal,
            synchronous: config.synchronous,
            is_memory,
            closed: AtomicBool::new(false),
            fault: None,
        };

        let write = pool.new_connection().await?;
        check_version(&write, config.version, is_new).await?;
        pool.fault = pool.verify_durability(&write).await?;
        *pool.write.lock().await = Some(write);
        Ok(pool)
    }

    /// Verify the durability pair took effect on the write connection —
    /// synchronous level governs commits and only the write connection
    /// commits; journal mode is a property of the database file. Read
    /// connections re-prove nothing and are not checked.
    ///
    /// Journal mode is only required when the config asked for WAL on a
    /// file-backed database (`:memory:` has no on-disk journal). The
    /// synchronous level is verified against [`Config::synchronous`] — a
    /// caller-supplied `pragma("synchronous", …)` diverging from it is applied
    /// (best-effort, after the verified pair) and then reported as exactly
    /// what it is: an effective level other than the one configured.
    ///
    /// Errs only on engine failure during readback; a mismatch is `Some`.
    async fn verify_durability(
        &self,
        write: &Connection,
    ) -> Result<Option<DurabilityFault>, BedrockError> {
        let state = match durability::read_state(write).await {
            Ok(state) => state,
            // An unreadable synchronous level is itself the fault.
            Err(BedrockError::DurabilityMismatch(fault)) => return Ok(Some(fault)),
            Err(error) => return Err(error),
        };

        // A non-WAL journal makes synchronous semantics moot; report it first.
        if self.wal_for_files && !self.is_memory && state.journal_mode != JournalMode::WAL {
            return Ok(Some(DurabilityFault {
                pragma: "journal_mode",
                requested: JournalMode::WAL.to_string(),
                effective: state.journal_mode.to_string(),
            }));
        }
        if state.synchronous != self.synchronous {
            return Ok(Some(DurabilityFault {
                pragma: "synchronous",
                requested: durability::sync_pragma_value(self.synchronous).into(),
                effective: durability::sync_pragma_value(state.synchronous).into(),
            }));
        }
        Ok(None)
    }

    /// The durability fault recorded at open, if any.
    pub(crate) fn fault(&self) -> Option<&DurabilityFault> {
        self.fault.as_ref()
    }

    /// Mark the pool closed and drop every pooled connection. Awaiting the write
    /// lock also waits out any in-flight write transaction.
    pub(crate) async fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        *self.write.lock().await = None;
        self.read_pool.lock().expect("read pool lock").clear();
    }

    /// Acquire the single write connection, first rolling back any transaction a
    /// previous holder left open.
    ///
    /// Refused on a durability-faulted pool: a mismatched territory is
    /// read-only, and every refusal carries the structured fault.
    pub(crate) async fn write(&self) -> Result<WriteGuard, BedrockError> {
        if let Some(fault) = &self.fault {
            return Err(BedrockError::DurabilityMismatch(fault.clone()));
        }
        self.write_unchecked().await
    }

    /// Acquire the write connection without the durability-fault check.
    ///
    /// For the open path's own bookkeeping DDL (idempotent meta tables the
    /// handle needs to function at all) and for durability readback — not for
    /// user data. Everything else goes through [`write`](Pool::write).
    pub(crate) async fn write_unchecked(&self) -> Result<WriteGuard, BedrockError> {
        self.check_open()?;
        let guard = self.write.clone().lock_owned().await;
        if guard.is_none() {
            return Err(BedrockError::Closed);
        }
        rollback_if_dirty(guard.as_ref().expect("checked Some")).await;
        Ok(WriteGuard { conn: guard })
    }

    /// Acquire a read connection, created lazily up to the configured ceiling;
    /// the semaphore provides backpressure once the ceiling is reached.
    pub(crate) async fn read(&self) -> Result<ReadGuard, BedrockError> {
        self.check_open()?;
        let permit = self
            .read_sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| BedrockError::Closed)?;
        let pooled = self.read_pool.lock().expect("read pool lock").pop();
        let conn = match pooled {
            Some(conn) => {
                rollback_if_dirty(&conn).await;
                conn
            }
            None => {
                debug_assert!(self.reads_created.load(Ordering::Relaxed) < self.read_limit);
                self.reads_created.fetch_add(1, Ordering::Relaxed);
                self.new_connection().await?
            }
        };
        Ok(ReadGuard {
            conn: Some(conn),
            pool: self.read_pool.clone(),
            _permit: permit,
        })
    }

    /// Create a fresh connection: the durability pair first (WAL on
    /// file-backed databases, then the synchronous level), caller-supplied
    /// pragmas after — so a caller can still override deliberately.
    async fn new_connection(&self) -> Result<Connection, BedrockError> {
        let conn = self.engine.connect()?;
        if self.wal_for_files && !self.is_memory {
            apply_pragma(&conn, "journal_mode", "WAL").await;
        }
        apply_pragma(
            &conn,
            "synchronous",
            durability::sync_pragma_value(self.synchronous),
        )
        .await;
        for (key, value) in &self.pragma {
            apply_pragma(&conn, key, value).await;
        }
        Ok(conn)
    }

    fn check_open(&self) -> Result<(), BedrockError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(BedrockError::Closed);
        }
        Ok(())
    }
}

/// Exclusive handle to the single write connection.
pub(crate) struct WriteGuard {
    conn: OwnedMutexGuard<Option<Connection>>,
}

impl std::ops::Deref for WriteGuard {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("checked Some at acquisition")
    }
}

/// Pooled read connection; returns itself to the pool on drop.
pub(crate) struct ReadGuard {
    conn: Option<Connection>,
    pool: Arc<std::sync::Mutex<Vec<Connection>>>,
    _permit: OwnedSemaphorePermit,
}

impl std::ops::Deref for ReadGuard {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("connection present until drop")
    }
}

impl Drop for ReadGuard {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.lock().expect("read pool lock").push(conn);
        }
    }
}

/// Roll back a connection left mid-transaction (a dropped, uncommitted tx).
async fn rollback_if_dirty(conn: &Connection) {
    if let Ok(false) = conn.is_autocommit() {
        let _ = conn.execute("ROLLBACK", ()).await;
    }
}

/// Apply one `PRAGMA k = v`, tolerating engine rejection with a warning.
///
/// The best-effort contract here covers *caller-supplied* pragmas only —
/// tuning knobs whose rejection is a legitimate engine-capability difference.
/// The durability pair (`journal_mode`, `synchronous`) also passes through
/// here, but is the exception: [`Pool::open`] reads it back and records a
/// [`DurabilityFault`] when it did not take effect, rather than leaving a log
/// line as the only evidence.
///
/// Uses `query` rather than `execute`: some setter pragmas (notably
/// `journal_mode`) return a result row, and turso's `execute` treats any row as
/// a `Misuse` error. Draining the rows applies the pragma and swallows the row.
async fn apply_pragma(conn: &Connection, key: &str, value: &str) {
    match conn.query(&format!("PRAGMA {key} = {value}"), ()).await {
        Ok(mut rows) => while let Ok(Some(_)) = rows.next().await {},
        Err(error) => {
            tracing::warn!(%key, %value, %error, "pragma rejected by turso");
        }
    }
}

/// Enforce `PRAGMA user_version`: a fresh database is stamped, an existing one
/// with a differing version is refused.
async fn check_version(conn: &Connection, version: i64, is_new: bool) -> Result<(), BedrockError> {
    if !is_new {
        let mut rows = conn.query("PRAGMA user_version", ()).await?;
        let got = match rows.next().await? {
            Some(row) => row.get_value(0)?.as_integer().copied().unwrap_or(0),
            None => 0,
        };
        if got != version {
            return Err(BedrockError::IncompatibleVersion { want: version, got });
        }
    }
    conn.execute(&format!("PRAGMA user_version = {version}"), ())
        .await?;
    Ok(())
}
