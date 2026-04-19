//! Seed phase orchestrator.
//!
//! Runs after refinery has applied all DDL migrations. Each seed is
//! self-idempotent (probes for a stable UUID before inserting) so there's no
//! separate `_seeds` tracking table.

use turso::Connection;

use crate::error::StoreResult;
use crate::migrations::seed_charts;
#[cfg(feature = "mock")]
use crate::migrations::seed_documents;

pub async fn run(conn: &Connection) -> StoreResult<()> {
    seed_charts::run(conn).await?;
    #[cfg(feature = "mock")]
    seed_documents::run(conn).await?;
    Ok(())
}
