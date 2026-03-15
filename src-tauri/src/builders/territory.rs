use tauri::{App, Manager};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

use crate::settings;
use crate::territory::{TerritoryManager, TerritoryRegistry};

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let managed_root = data_dir.join("territories");
    tracing::debug!(path = %managed_root.display(), "territory managed root");

    let manager = TerritoryManager::new(managed_root);

    // Try to resume the last active workspace
    tracing::debug!("opening settings store");
    let store = app.store(settings::SETTINGS_FILE)?;
    tracing::debug!("reading territory registry from store");
    let registry: TerritoryRegistry = store
        .get(settings::KEY_TERRITORIES)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    store.close_resource();

    tracing::debug!(
        count = registry.registry.len(),
        last_active = ?registry.last_active_id,
        "loaded territory registry"
    );

    if let Some(last_id) = registry.last_active_id {
        if let Some(entry) = registry.registry.iter().find(|e| e.id == last_id) {
            let workspace_dir = manager.entry_path(entry);
            tracing::info!(id = %last_id, path = %workspace_dir.display(), "resuming territory");

            let db_path = workspace_dir.join("waypoints.db");
            tracing::debug!("block_on: opening database");
            match tauri::async_runtime::block_on(store::open(&db_path)) {
                Ok(db) => {
                    tracing::debug!("block_on: db opened, acquiring locks");
                    let active_db = manager.active_db.clone();
                    let active_id = manager.active_id.clone();
                    tauri::async_runtime::block_on(async {
                        *active_db.lock().await = Some(db);
                        *active_id.lock().await = Some(last_id);
                    });
                    tracing::info!("resumed territory {last_id}");
                }
                Err(e) => {
                    tracing::warn!("failed to resume territory {last_id}: {e}");
                }
            }
        } else {
            tracing::warn!(id = %last_id, "last_active_id not found in registry, skipping resume");
        }
    } else {
        tracing::debug!("no last_active_id, skipping territory resume");
    }

    // EditorService shares the same Arc<Mutex<Option<DB>>> as the manager.
    let editor = editor::EditorService::new(manager.active_db.clone());
    app.manage(Mutex::new(editor));
    tracing::debug!("editor service registered in app state");

    app.manage(manager);
    tracing::debug!("territory manager registered in app state");

    Ok(())
}
