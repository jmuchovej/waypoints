use chrono::Utc;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::settings;
use crate::territory::{
    Territory, TerritoryError, TerritoryKind, TerritoryManager, TerritoryRegistry, TerritoryResult,
};

fn load_registry(app: &AppHandle) -> TerritoryResult<TerritoryRegistry> {
    let store = app
        .store(settings::SETTINGS_FILE)
        .map_err(|e| TerritoryError::Config(e.to_string()))?;

    let value = store
        .get(settings::KEY_TERRITORIES)
        .unwrap_or_else(|| serde_json::to_value(TerritoryRegistry::default()).unwrap());

    store.close_resource();

    serde_json::from_value(value).map_err(|e| TerritoryError::Config(e.to_string()))
}

fn save_registry(app: &AppHandle, registry: &TerritoryRegistry) -> TerritoryResult<()> {
    let store = app
        .store(settings::SETTINGS_FILE)
        .map_err(|e| TerritoryError::Config(e.to_string()))?;

    let value =
        serde_json::to_value(registry).map_err(|e| TerritoryError::Config(e.to_string()))?;
    store.set(settings::KEY_TERRITORIES, value);
    store
        .save()
        .map_err(|e| TerritoryError::Config(e.to_string()))?;
    store.close_resource();

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn territory_list(app: AppHandle) -> TerritoryResult<Vec<Territory>> {
    let registry = load_registry(&app)?;
    Ok(registry.registry)
}

#[tauri::command]
#[specta::specta]
pub async fn territory_get_active(
    app: AppHandle,
    mgr: State<'_, TerritoryManager>,
) -> TerritoryResult<Option<Territory>> {
    let active_id = mgr.active_id.lock().await;
    let Some(id) = *active_id else {
        return Ok(None);
    };

    let registry = load_registry(&app)?;
    Ok(registry.registry.into_iter().find(|e| e.id == id))
}

#[tauri::command]
#[specta::specta]
pub async fn territory_create(
    app: AppHandle,
    mgr: State<'_, TerritoryManager>,
    editor_state: State<'_, Mutex<editor::EditorService>>,
    name: String,
) -> TerritoryResult<Territory> {
    let id = Uuid::new_v4();
    let workspace_dir = mgr.managed_path(id);
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

    let mut registry = load_registry(&app)?;
    registry.registry.push(entry.clone());
    registry.last_active_id = Some(id);
    save_registry(&app, &registry)?;

    // Checkpoint and deactivate old workspace if any
    {
        let mut active_db = mgr.active_db.lock().await;
        if let Some(old_db) = active_db.take() {
            store::checkpoint(&old_db).await;
        }
        *active_db = Some(db.clone());
    }
    *mgr.active_id.lock().await = Some(id);

    // Replace EditorService with new DB
    *editor_state.lock().await = editor::EditorService::new(db);

    Ok(entry)
}

#[tauri::command]
#[specta::specta]
pub async fn territory_open(
    app: AppHandle,
    mgr: State<'_, TerritoryManager>,
    editor_state: State<'_, Mutex<editor::EditorService>>,
    id: Option<Uuid>,
    path: Option<String>,
    name: Option<String>,
) -> TerritoryResult<Territory> {
    let mut registry = load_registry(&app)?;

    let entry = if let Some(id) = id {
        // Open existing registered workspace by ID
        let entry = registry
            .registry
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or(TerritoryError::NotFound(id))?;
        entry.last_opened_at = Some(Utc::now());
        entry.clone()
    } else if let Some(ref ext_path) = path {
        // Open or register an external workspace by path
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

    let workspace_dir = mgr.entry_path(&entry);
    let db_path = workspace_dir.join("waypoints.db");
    let db = store::open(&db_path).await?;

    registry.last_active_id = Some(entry.id);
    save_registry(&app, &registry)?;

    // Checkpoint old workspace, activate new
    {
        let mut active_db = mgr.active_db.lock().await;
        if let Some(old_db) = active_db.take() {
            store::checkpoint(&old_db).await;
        }
        *active_db = Some(db.clone());
    }
    *mgr.active_id.lock().await = Some(entry.id);

    // Replace EditorService with new DB
    *editor_state.lock().await = editor::EditorService::new(db);

    Ok(entry)
}

#[tauri::command]
#[specta::specta]
pub async fn territory_close(
    app: AppHandle,
    mgr: State<'_, TerritoryManager>,
    editor_state: State<'_, Mutex<editor::EditorService>>,
) -> TerritoryResult<()> {
    let mut active_db = mgr.active_db.lock().await;
    if let Some(db) = active_db.take() {
        store::checkpoint(&db).await;
    }
    *mgr.active_id.lock().await = None;

    let mut registry = load_registry(&app)?;
    registry.last_active_id = None;
    save_registry(&app, &registry)?;

    // Replace EditorService with placeholder
    let placeholder_db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .map_err(|e| TerritoryError::Database(e))?;
    *editor_state.lock().await = editor::EditorService::new(placeholder_db);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn territory_remove(
    app: AppHandle,
    mgr: State<'_, TerritoryManager>,
    editor_state: State<'_, Mutex<editor::EditorService>>,
    id: Uuid,
    delete_data: bool,
) -> TerritoryResult<()> {
    let mut registry = load_registry(&app)?;

    let entry_idx = registry
        .registry
        .iter()
        .position(|e| e.id == id)
        .ok_or(TerritoryError::NotFound(id))?;

    let entry = registry.registry.remove(entry_idx);

    // If removing the active workspace, deactivate it first
    {
        let current_id = *mgr.active_id.lock().await;
        if current_id == Some(id) {
            let mut active_db = mgr.active_db.lock().await;
            if let Some(db) = active_db.take() {
                store::checkpoint(&db).await;
            }
            *mgr.active_id.lock().await = None;

            // Replace EditorService with placeholder
            let placeholder_db = sea_orm::Database::connect("sqlite::memory:")
                .await
                .map_err(|e| TerritoryError::Database(e))?;
            *editor_state.lock().await = editor::EditorService::new(placeholder_db);
        }
    }

    if registry.last_active_id == Some(id) {
        registry.last_active_id = None;
    }
    save_registry(&app, &registry)?;

    // Only delete files for managed workspaces
    if delete_data && matches!(entry.kind, TerritoryKind::Managed) {
        let workspace_dir = mgr.managed_path(entry.id);
        if workspace_dir.exists() {
            std::fs::remove_dir_all(&workspace_dir)?;
        }
    }

    Ok(())
}
