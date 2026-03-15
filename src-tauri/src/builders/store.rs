use tauri::App;
use tauri_plugin_store::StoreExt;

use crate::settings;
use crate::territory::TerritoryRegistry;

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("opening settings store: {}", settings::SETTINGS_FILE);
    let store = app.store(settings::SETTINGS_FILE)?;

    store.save()?;
    store.close_resource();

    Ok(())
}
