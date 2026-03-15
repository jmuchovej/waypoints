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

    tracing::info!("running migrations");
    Migrator::up(&db, None).await?;
    tracing::debug!("migrations complete");

    Ok(db)
}

pub async fn reset(db: DatabaseConnection) -> StoreResult<DatabaseConnection> {
    tracing::info!("resetting migrations");
    Migrator::down(&db, None).await?;
    tracing::debug!("  migration reset complete");

    tracing::info!("running migrations");
    Migrator::up(&db, None).await?;
    tracing::debug!("  migrations complete");

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
