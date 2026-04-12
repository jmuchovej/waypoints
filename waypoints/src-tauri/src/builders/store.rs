use tauri::App;
use tauri_plugin_store::StoreExt;

use crate::settings;
use crate::territory::TerritoryRegistry;

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("opening settings store: {}", settings::SETTINGS_FILE);
    let store = app.store(settings::SETTINGS_FILE)?;

    if store.get(settings::KEY_TERRITORIES).is_none() {
        tracing::debug!("no territory registry found, writing default");
        let default = serde_json::to_value(TerritoryRegistry::default())?;
        store.set(settings::KEY_TERRITORIES, default);
    } else {
        tracing::debug!("territory registry already exists in store");
    }

    store.save()?;
    store.close_resource();

    Ok(())
}
