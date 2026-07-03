use std::path::Path;

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

use crate::error::StoreResult;
use crate::migrations;

pub async fn open(path: &Path) -> StoreResult<DatabaseConnection> {
    if let Some(parent) = path.parent() {
        tracing::debug!(path = %parent.display(), "ensuring parent directory exists");
        std::fs::create_dir_all(parent)?;
    }

    let path_str = path.display().to_string();
    let db_exists = path.exists();

    tracing::info!(path = %path_str, exists = db_exists, "running migrations via turso");
    run_migrations(&path_str).await?;
    tracing::debug!("migrations complete");

    let db_url = format!("sqlite:{}?mode=rwc", path_str);
    tracing::info!(url = %db_url, "opening sea-orm query connection");
    let db = Database::connect(&db_url).await?;
    apply_pragmas(&db).await?;
    tracing::debug!("sea-orm connection ready");

    Ok(db)
}

/// Drop all application tables (including refinery's history), re-run
/// migrations via turso, and return a fresh sea-orm connection.
///
/// Caller must hand over the path the territory was opened with; sea-orm
/// doesn't expose the underlying file path.
pub async fn reset(old_db: DatabaseConnection, path: &Path) -> StoreResult<DatabaseConnection> {
    tracing::info!(path = %path.display(), "resetting database");

    // Close the existing sea-orm connection so turso can hold the file.
    drop(old_db);

    let path_str = path.display().to_string();
    drop_all_tables(&path_str).await?;
    run_migrations(&path_str).await?;

    let db_url = format!("sqlite:{}?mode=rwc", path_str);
    let db = Database::connect(&db_url).await?;
    apply_pragmas(&db).await?;
    tracing::info!("database reset complete");

    Ok(db)
}

pub async fn checkpoint(db: &DatabaseConnection) {
    let result = db
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "PRAGMA wal_checkpoint(TRUNCATE)",
        ))
        .await;

    match result {
        Ok(_) => tracing::info!("WAL checkpoint completed"),
        Err(e) => tracing::warn!("WAL checkpoint failed: {e}"),
    }
}

async fn run_migrations(path_str: &str) -> StoreResult<()> {
    let db = turso::Builder::new_local(path_str).build().await?;
    let conn = db.connect()?;
    // Persistent pragmas (WAL is per-file) so the first sea-orm connection
    // inherits them. `pragma_update` reads back the resulting row;
    // `execute` would error because PRAGMA returns rows.
    conn.pragma_update("journal_mode", "WAL").await?;
    migrations::run(&conn).await?;
    Ok(())
}

async fn drop_all_tables(path_str: &str) -> StoreResult<()> {
    let db = turso::Builder::new_local(path_str).build().await?;
    let conn = db.connect()?;
    conn.execute_batch(
        "DROP TABLE IF EXISTS \"terrains\"; \
         DROP TABLE IF EXISTS \"charts\"; \
         DROP TABLE IF EXISTS \"marks\"; \
         DROP TABLE IF EXISTS \"refinery_schema_history\";",
    )
    .await?;
    Ok(())
}

async fn apply_pragmas(db: &DatabaseConnection) -> StoreResult<()> {
    for (pragma, value) in [
        ("page_size", "8192"), // 8 KB (persistent; no-op if tables exist)
        ("journal_mode", "WAL"),
        ("synchronous", "NORMAL"),
        ("foreign_keys", "ON"),
        ("busy_timeout", "5000"),
        ("cache_size", "-65536"), // 64 MB
        ("temp_store", "MEMORY"),
        ("mmap_size", "134217728"), // 128 MB
    ] {
        db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!("PRAGMA {pragma} = {value}"),
        ))
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn count(db: &DatabaseConnection, table: &str) -> i64 {
        let row = db
            .query_one(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                format!("SELECT COUNT(*) AS c FROM \"{table}\""),
            ))
            .await
            .unwrap()
            .unwrap();
        row.try_get_by("c").unwrap()
    }

    /// End-to-end smoke test: `open()` must run refinery migrations,
    /// seed builtins, and return a working SeaORM connection. Regression
    /// guard for the kind of silent-failure (e.g. `PRAGMA` via `execute`)
    /// that leaves the DB file at 0 bytes.
    #[tokio::test]
    async fn open_runs_migrations_and_seeds() {
        let dir = tempdir().unwrap();
        let db = open(&dir.path().join("test.db")).await.expect("open");

        assert_eq!(count(&db, "refinery_schema_history").await, 6);
        assert_eq!(count(&db, "charts").await, 9);

        // mock feature also seeds documents; without it, marks/terrains are 0.
        #[cfg(feature = "mock")]
        {
            assert!(count(&db, "marks").await > 0);
            assert!(count(&db, "terrains").await > 0);
        }
    }
}
