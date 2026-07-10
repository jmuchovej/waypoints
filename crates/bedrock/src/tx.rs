//! Explicit read and write transactions over pooled connections.
//!
//! A [`ReadTx`] wraps `BEGIN`; a [`WriteTx`] wraps `BEGIN IMMEDIATE` and can
//! nest via [`WriteTx::savepoint`]. Both expose `execute`/`query` for raw SQL.
//! Committing is explicit; a handle dropped without `commit` leaves its
//! connection mid-transaction, and the pool rolls it back on next acquisition
//! (see [`crate::pool`]).

use std::sync::atomic::{AtomicU64, Ordering};

use turso::{IntoParams, Rows};

use crate::error::BedrockError;
use crate::pool::{ReadGuard, WriteGuard};

/// Distinct savepoint names, so nested savepoints on one connection never
/// collide. Monotonic for the process; savepoints are released in LIFO order.
static SAVEPOINT_IDS: AtomicU64 = AtomicU64::new(0);

/// A read-only transaction (`BEGIN`), holding a pooled read connection.
pub struct ReadTx {
    conn: ReadGuard,
    done: bool,
}

impl ReadTx {
    pub(crate) async fn begin(conn: ReadGuard) -> Result<ReadTx, BedrockError> {
        conn.execute("BEGIN", ()).await?;
        Ok(ReadTx { conn, done: false })
    }

    /// Run a query within the transaction.
    pub async fn query(&self, sql: &str, params: impl IntoParams) -> Result<Rows, BedrockError> {
        Ok(self.conn.query(sql, params).await?)
    }

    /// End the transaction. A handle dropped before this is rolled back lazily.
    pub async fn commit(mut self) -> Result<(), BedrockError> {
        self.done = true;
        self.conn.execute("COMMIT", ()).await?;
        Ok(())
    }

    /// Whether the transaction has been completed.
    pub fn done(&self) -> bool {
        self.done
    }
}

/// A read-write transaction (`BEGIN IMMEDIATE`), holding the write connection.
pub struct WriteTx {
    conn: WriteGuard,
    done: bool,
}

impl WriteTx {
    pub(crate) async fn begin(conn: WriteGuard) -> Result<WriteTx, BedrockError> {
        conn.execute("BEGIN IMMEDIATE", ()).await?;
        Ok(WriteTx { conn, done: false })
    }

    /// Execute a statement, returning the number of affected rows.
    pub async fn execute(&self, sql: &str, params: impl IntoParams) -> Result<u64, BedrockError> {
        Ok(self.conn.execute(sql, params).await?)
    }

    /// Run a query within the transaction.
    pub async fn query(&self, sql: &str, params: impl IntoParams) -> Result<Rows, BedrockError> {
        Ok(self.conn.query(sql, params).await?)
    }

    /// Commit the transaction.
    pub async fn commit(mut self) -> Result<(), BedrockError> {
        self.done = true;
        self.conn.execute("COMMIT", ()).await?;
        Ok(())
    }

    /// Roll the transaction back.
    pub async fn rollback(mut self) -> Result<(), BedrockError> {
        self.done = true;
        self.conn.execute("ROLLBACK", ()).await?;
        Ok(())
    }

    /// Whether the transaction has been completed.
    pub fn done(&self) -> bool {
        self.done
    }

    /// Open a nested transaction backed by a `SAVEPOINT`.
    pub async fn savepoint(&mut self) -> Result<Savepoint<'_>, BedrockError> {
        let id = SAVEPOINT_IDS.fetch_add(1, Ordering::Relaxed) + 1;
        self.conn.execute(&format!("SAVEPOINT sp{id}"), ()).await?;
        Ok(Savepoint { tx: self, id })
    }
}

/// A nested write transaction. Release it with [`Savepoint::commit`] or undo it
/// with [`Savepoint::rollback`]; dropping it keeps its writes in the outer
/// transaction.
pub struct Savepoint<'a> {
    tx: &'a mut WriteTx,
    id: u64,
}

impl Savepoint<'_> {
    /// Execute a statement within the savepoint.
    pub async fn execute(&self, sql: &str, params: impl IntoParams) -> Result<u64, BedrockError> {
        Ok(self.tx.conn.execute(sql, params).await?)
    }

    /// Run a query within the savepoint.
    pub async fn query(&self, sql: &str, params: impl IntoParams) -> Result<Rows, BedrockError> {
        Ok(self.tx.conn.query(sql, params).await?)
    }

    /// Release (commit) the savepoint into the outer transaction.
    pub async fn commit(self) -> Result<(), BedrockError> {
        self.tx
            .conn
            .execute(&format!("RELEASE SAVEPOINT sp{}", self.id), ())
            .await?;
        Ok(())
    }

    /// Roll back to the savepoint, undoing writes made since it opened.
    pub async fn rollback(self) -> Result<(), BedrockError> {
        self.tx
            .conn
            .execute(&format!("ROLLBACK TO SAVEPOINT sp{}", self.id), ())
            .await?;
        Ok(())
    }
}
