use async_trait::async_trait;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use cartograph::territory::{RegistryStore, TerritoryError, TerritoryRegistry, TerritoryResult};

use crate::settings;

pub(crate) struct TauriRegistryStore {
    app: AppHandle,
}

impl TauriRegistryStore {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// Ensure the settings store has a default territory registry if absent.
    pub fn ensure_defaults(&self) -> Result<(), Box<dyn std::error::Error>> {
        let store = self.app.store(settings::SETTINGS_FILE)?;

        if store.get(settings::KEY_TERRITORIES).is_none() {
            tracing::debug!("no territory registry found, writing default");
            let default = serde_json::to_value(TerritoryRegistry::default())?;
            store.set(settings::KEY_TERRITORIES, default);
        }

        store.save()?;
        store.close_resource();
        Ok(())
    }
}

#[async_trait]
impl RegistryStore for TauriRegistryStore {
    async fn load(&self) -> TerritoryResult<TerritoryRegistry> {
        let store = self
            .app
            .store(settings::SETTINGS_FILE)
            .map_err(|e| TerritoryError::Config(e.to_string()))?;

        let value = store
            .get(settings::KEY_TERRITORIES)
            .unwrap_or_else(|| serde_json::to_value(TerritoryRegistry::default()).unwrap());

        store.close_resource();

        serde_json::from_value(value).map_err(|e| TerritoryError::Config(e.to_string()))
    }

    async fn save(&self, registry: &TerritoryRegistry) -> TerritoryResult<()> {
        let store = self
            .app
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
}
