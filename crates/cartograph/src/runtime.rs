use crate::{
    ChartHandle, EditorHandle, MarkHandle, RegistryStore, SyncService, TerritoryHandle,
    TerritoryResult, editor::EditorService, territory::TerritoryManager,
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

pub struct WaypointsRuntime {
    manager: TerritoryManager,
    pub(crate) editor: Mutex<EditorService>,
    registry: Arc<dyn RegistryStore>,
    sync: SyncService,
}

impl WaypointsRuntime {
    pub fn new(
        managed_root: PathBuf,
        sync_root: PathBuf,
        registry: Arc<dyn RegistryStore>,
    ) -> Self {
        let manager = TerritoryManager::new(managed_root);
        let editor = EditorService::new(manager.active_db.clone());
        Self {
            manager,
            editor: Mutex::new(editor),
            registry,
            sync: SyncService::new(sync_root),
        }
    }

    pub async fn resume(&self) -> TerritoryResult<()> {
        let registry = self.registry.load().await?;

        let Some(last_id) = registry.last_active_id else {
            tracing::debug!("no last_active_id, skipping territory resume");
            return Ok(());
        };

        let Some(entry) = registry.registry.iter().find(|e| e.id == last_id) else {
            tracing::warn!(id = %last_id, "last_active_id not found in registry, skipping resume");
            return Ok(());
        };

        let workspace_dir = self.manager.entry_path(entry);
        tracing::info!(id = %last_id, path = %workspace_dir.display(), "resuming territory");

        let db_path = workspace_dir.join("waypoints.db");
        match store::open(&db_path).await {
            Ok(db) => {
                *self.manager.active_db.lock().await = Some(db);
                *self.manager.active_id.lock().await = Some(last_id);
                tracing::info!("resumed territory {last_id}");
            }
            Err(e) => {
                tracing::warn!("failed to resume territory {last_id}: {e}");
            }
        }

        Ok(())
    }

    pub fn editor(&self) -> EditorHandle<'_> {
        EditorHandle {
            editor: &self.editor,
        }
    }

    pub fn territory(&self) -> TerritoryHandle<'_> {
        TerritoryHandle {
            manager: &self.manager,
            registry: self.registry.as_ref(),
            editor: &self.editor,
        }
    }

    pub fn sync(&self) -> &SyncService {
        &self.sync
    }

    pub fn marks(&self) -> MarkHandle<'_> {
        MarkHandle {
            db: &self.manager.active_db,
        }
    }

    pub fn charts(&self) -> ChartHandle<'_> {
        ChartHandle {
            db: &self.manager.active_db,
        }
    }

    /// Reset the active database (drop all tables, re-run migrations).
    /// Clears editor undo histories afterward.
    pub async fn reset_db(&self) -> Result<(), store::StoreError> {
        let Some(active_id) = *self.manager.active_id.lock().await else {
            tracing::warn!("reset_db: no active territory");
            return Ok(());
        };

        let registry = self
            .registry
            .load()
            .await
            .map_err(|e| store::StoreError::Custom(format!("failed to load registry: {e}")))?;
        let Some(entry) = registry.registry.iter().find(|e| e.id == active_id) else {
            tracing::warn!(id = %active_id, "reset_db: active territory missing from registry");
            return Ok(());
        };
        let db_path = self.manager.entry_path(entry).join("waypoints.db");

        let mut db_guard = self.manager.active_db.lock().await;
        let Some(db) = db_guard.take() else {
            tracing::warn!("reset_db: no active db");
            return Ok(());
        };

        match store::reset(db, &db_path).await {
            Ok(fresh_db) => {
                *db_guard = Some(fresh_db);
                drop(db_guard);
                self.editor.lock().await.clear_histories();
                tracing::info!("database reset complete");
                Ok(())
            }
            Err(e) => {
                // DB is gone — leave the slot empty
                tracing::error!("database reset failed: {e}");
                Err(e)
            }
        }
    }
}
