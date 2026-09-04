use std::sync::Arc;

use crate::collections;
use crate::error::{StoreError, StoreResult};

/// Bedrock-backed store. The `db` handle is the single point of contact with
/// the turso engine; all collection operations go through it.
///
/// Cheaply cloneable: the inner `Arc` lets the shared database pool be held by
/// multiple owners (e.g. `TerritoryManager` and the per-handle structs in
/// cartograph) without requiring the caller to wrap in a second Arc.
#[derive(Clone)]
pub struct Store {
    db: Arc<bedrock::Database>,
}

impl Store {
    pub fn db(&self) -> &bedrock::Database {
        &self.db
    }

    /// Consume `self` and return the underlying `bedrock::Database` handle.
    /// Only succeeds if this is the last owner; otherwise returns the Store back.
    pub fn try_into_db(self) -> Result<bedrock::Database, Self> {
        Arc::try_unwrap(self.db).map_err(|db| Store { db })
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
        db.ensure_collection::<domain::Mark>().await?;
        db.ensure_collection::<domain::Chart>().await?;
        db.ensure_collection::<domain::Terrain>().await?;
        db.ensure_collection::<crate::query::sync::SyncWinner>()
            .await?;
        db.ensure_collection::<crate::query::sync::SyncFoldState>()
            .await?;
        db.ensure_collection::<crate::query::sync::SyncOutbox>()
            .await?;
        db.ensure_collection::<crate::query::sync::SyncTrack>()
            .await?;
        db.ensure_collection::<crate::query::sync::SyncDeferred>()
            .await?;
        collections::seed::run(&db).await?;

        // Dev-only: seed the demo document corpus so a fresh DB has something
        // to work with. Runs after chart seeding (readings resolve against
        // chart features) and is idempotent.
        #[cfg(feature = "mock")]
        collections::demo::run(&db).await?;

        Ok(Store { db: Arc::new(db) })
    }
}
