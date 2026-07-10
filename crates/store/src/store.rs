use crate::collections;
use crate::error::{StoreError, StoreResult};

/// Bedrock-backed store. The `db` handle is the single point of contact with
/// the turso engine; all collection operations go through it.
///
/// This runs ALONGSIDE the sea-orm stack — `store::open` (in `tools.rs`)
/// continues to return a `DatabaseConnection`. `Store::open` here is the
/// bedrock replacement, wired in parallel until the swap is ready.
pub struct Store {
    db: bedrock::Database,
}

impl Store {
    pub fn db(&self) -> &bedrock::Database {
        &self.db
    }

    /// Open (or create) the bedrock store at `path`.
    ///
    /// Ensures each collection's table exists (safe-additive DDL), then seeds
    /// the builtin charts. Idempotent — safe to call on every boot.
    pub async fn open(path: &std::path::Path) -> StoreResult<Store> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = bedrock::Database::open(&path.display().to_string()).await?;
        db.ensure_collection::<collections::MarkRow>().await?;
        db.ensure_collection::<collections::ChartRow>().await?;
        db.ensure_collection::<collections::TerrainRow>().await?;
        collections::seed::run(&db).await?;

        Ok(Store { db })
    }
}
