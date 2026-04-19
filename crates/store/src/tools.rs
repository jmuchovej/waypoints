use std::path::Path;

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

use crate::error::StoreResult;
use crate::migrations::{Migrator, MigratorTrait};

pub async fn open(path: &Path) -> StoreResult<DatabaseConnection> {
    if let Some(parent) = path.parent() {
        tracing::debug!(path = %parent.display(), "ensuring parent directory exists");
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite:{}?mode=rwc", path.display());
    let db_exists = path.exists();

    tracing::info!(url = %db_url, exists = db_exists, "connecting to database");
    let db = Database::connect(&db_url).await?;
    tracing::debug!("database connection established");

    for (pragma, value) in [
        ("page_size", "8192"), // 8 KB
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
            format!("PRAGMA {} = {}", pragma, value),
        ))
        .await?;
    }
    tracing::debug!("SQLite pragmas configured");

    tracing::info!("running migrations");
    Migrator::up(&db, None).await?;
    tracing::debug!("migrations complete");

    Ok(db)
}

pub async fn reset(db: DatabaseConnection) -> StoreResult<DatabaseConnection> {
    tracing::info!("resetting database (fresh)");
    Migrator::fresh(&db).await?;
    tracing::debug!("  fresh reset complete");

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
