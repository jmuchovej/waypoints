use std::path::Path;

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};
use waypoints_migrations::{Migrator, MigratorTrait};

use super::TerritoryResult;

pub async fn open(workspace_dir: &Path) -> TerritoryResult<DatabaseConnection> {
    tracing::debug!(path = %workspace_dir.display(), "ensuring workspace directory exists");
    std::fs::create_dir_all(workspace_dir)?;

    let db_path = workspace_dir.join("waypoints.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let db_exists = db_path.exists();

    tracing::info!(url = %db_url, exists = db_exists, "connecting to database");
    let db = Database::connect(&db_url).await?;
    tracing::debug!("database connection established");

    tracing::info!("running migrations");
    Migrator::up(&db, None).await?;
    tracing::debug!("migrations complete");

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
