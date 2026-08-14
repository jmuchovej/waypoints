use super::TerritoryManager;
use crate::{
    RegistryStore, Territory, TerritoryError, TerritoryKind, TerritoryResult, editor::EditorService,
};
use chrono::Utc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct TerritoryHandle<'a> {
    pub(crate) manager: &'a TerritoryManager,
    pub(crate) registry: &'a dyn RegistryStore,
    pub(crate) editor: &'a Mutex<EditorService>,
}

impl TerritoryHandle<'_> {
    pub async fn list(&self) -> TerritoryResult<Vec<Territory>> {
        let registry = self.registry.load().await?;
        Ok(registry.registry)
    }

    pub async fn get_active(&self) -> TerritoryResult<Option<Territory>> {
        let active_id = self.manager.active_id.lock().await;
        let Some(id) = *active_id else {
            return Ok(None);
        };

        let registry = self.registry.load().await?;
        Ok(registry.registry.into_iter().find(|e| e.id == id))
    }

    pub async fn create(&self, name: String) -> TerritoryResult<Territory> {
        let id = Uuid::new_v4();
        let workspace_dir = self.manager.managed_path(id);
        let db_path = workspace_dir.join("waypoints.db");

        let db = store::open(&db_path).await?;

        let entry = Territory {
            id,
            name,
            kind: TerritoryKind::Managed,
            path: None,
            created_at: Utc::now(),
            last_opened_at: Some(Utc::now()),
        };

        let mut registry = self.registry.load().await?;
        registry.registry.push(entry.clone());
        registry.last_active_id = Some(id);
        self.registry.save(&registry).await?;

        // Checkpoint and deactivate old workspace if any
        {
            let mut active_db = self.manager.active_db.lock().await;
            if let Some(old_db) = active_db.take() {
                store::checkpoint(&old_db).await;
            }
            *active_db = Some(db);
        }
        *self.manager.active_id.lock().await = Some(id);

        // Clear undo histories for the new territory
        self.editor.lock().await.clear_histories();

        Ok(entry)
    }

    pub async fn open(
        &self,
        id: Option<Uuid>,
        path: Option<String>,
        name: Option<String>,
    ) -> TerritoryResult<Territory> {
        let mut registry = self.registry.load().await?;

        let entry = if let Some(id) = id {
            let entry = registry
                .registry
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(TerritoryError::NotFound(id))?;
            entry.last_opened_at = Some(Utc::now());
            entry.clone()
        } else if let Some(ref ext_path) = path {
            if let Some(existing) = registry.registry.iter_mut().find(|e| {
                matches!(e.kind, TerritoryKind::External) && e.path.as_deref() == Some(ext_path)
            }) {
                existing.last_opened_at = Some(Utc::now());
                existing.clone()
            } else {
                let entry = Territory {
                    id: Uuid::new_v4(),
                    name: name.unwrap_or_else(|| {
                        std::path::Path::new(ext_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Workspace")
                            .to_string()
                    }),
                    kind: TerritoryKind::External,
                    path: Some(ext_path.clone()),
                    created_at: Utc::now(),
                    last_opened_at: Some(Utc::now()),
                };
                registry.registry.push(entry.clone());
                entry
            }
        } else {
            return Err(TerritoryError::Config(
                "must provide either id or path".to_string(),
            ));
        };

        let workspace_dir = self.manager.entry_path(&entry);
        let db_path = workspace_dir.join("waypoints.db");
        let db = store::open(&db_path).await?;

        registry.last_active_id = Some(entry.id);
        self.registry.save(&registry).await?;

        // Checkpoint old workspace, activate new
        {
            let mut active_db = self.manager.active_db.lock().await;
            if let Some(old_db) = active_db.take() {
                store::checkpoint(&old_db).await;
            }
            *active_db = Some(db);
        }
        *self.manager.active_id.lock().await = Some(entry.id);

        // Clear undo histories for the new territory
        self.editor.lock().await.clear_histories();

        Ok(entry)
    }

    pub async fn close(&self) -> TerritoryResult<()> {
        let mut active_db = self.manager.active_db.lock().await;
        if let Some(db) = active_db.take() {
            store::checkpoint(&db).await;
        }
        *self.manager.active_id.lock().await = None;

        let mut registry = self.registry.load().await?;
        registry.last_active_id = None;
        self.registry.save(&registry).await?;

        // Clear undo histories — EditorService will return NoActiveTerritory
        // on subsequent calls since the shared Arc now holds None.
        self.editor.lock().await.clear_histories();

        Ok(())
    }

    pub async fn remove(&self, id: Uuid, delete_data: bool) -> TerritoryResult<()> {
        let mut registry = self.registry.load().await?;

        let entry_idx = registry
            .registry
            .iter()
            .position(|e| e.id == id)
            .ok_or(TerritoryError::NotFound(id))?;

        let entry = registry.registry.remove(entry_idx);

        // If removing the active workspace, deactivate it first
        {
            let current_id = *self.manager.active_id.lock().await;
            if current_id == Some(id) {
                let mut active_db = self.manager.active_db.lock().await;
                if let Some(db) = active_db.take() {
                    store::checkpoint(&db).await;
                }
                *self.manager.active_id.lock().await = None;

                self.editor.lock().await.clear_histories();
            }
        }

        if registry.last_active_id == Some(id) {
            registry.last_active_id = None;
        }
        self.registry.save(&registry).await?;

        // Only delete files for managed workspaces
        if delete_data && matches!(entry.kind, TerritoryKind::Managed) {
            let workspace_dir = self.manager.managed_path(entry.id);
            if workspace_dir.exists() {
                std::fs::remove_dir_all(&workspace_dir)?;
            }
        }

        Ok(())
    }
}
