//! Database migration entry point.
//!
//! Two phases:
//!
//! 1. **DDL** via refinery — SQL files in `migrations/sql/`, embedded at
//!    compile time, tracked in `refinery_schema_history`. A custom
//!    [`turso_adapter::TursoConn`] bridges refinery's async traits to the
//!    [`turso::Connection`].
//! 2. **Seeds** via [`seeds::run`] — runtime Rust seeders (charts, mock
//!    documents) that probe for stable UUIDs and skip themselves on
//!    subsequent boots.

mod seed_charts;
#[cfg(feature = "mock")]
mod seed_documents;
mod seeds;
mod turso_adapter;

use turso::Connection;

use crate::error::StoreResult;

mod embedded {
    refinery::embed_migrations!("src/migrations/sql");
}

pub async fn run(conn: &Connection) -> StoreResult<()> {
    let mut adapter = turso_adapter::TursoConn(conn.clone());
    let report = embedded::migrations::runner()
        .run_async(&mut adapter)
        .await?;
    tracing::info!(
        applied = report.applied_migrations().len(),
        "refinery: ddl migrations applied"
    );

    seeds::run(conn).await?;
    Ok(())
}
