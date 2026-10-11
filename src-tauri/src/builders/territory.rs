use crate::registry_store::TauriRegistryStore;
use cartograph::WaypointsRuntime;
use std::sync::Arc;
use tauri::{App, Manager};

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let managed_root = data_dir.join("territories");
    let sync_root = data_dir.join("sync");
    tracing::debug!(path = %managed_root.display(), "territory managed root");

    let registry_store = Arc::new(TauriRegistryStore::new(app.handle().clone()));

    // Ensure settings store has default registry
    registry_store.ensure_defaults()?;

    let rt = WaypointsRuntime::new(managed_root, sync_root, registry_store);

    // Try to resume the last active workspace
    tauri::async_runtime::block_on(async {
        if let Err(e) = rt.resume().await {
            tracing::warn!("failed to resume territory: {e}");
        }
        // Maintain the live sync subscription (fold on doorbells, reconnect
        // on dead sessions) for the app's lifetime.
        rt.sync_service().start_live();
    });

    app.manage(Arc::new(rt));
    tracing::debug!("waypoints runtime registered in app state");

    Ok(())
}
