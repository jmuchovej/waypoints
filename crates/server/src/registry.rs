//! File-backed [`RegistryStore`] for the dev server.
//!
//! Reads and writes the SAME `settings.json` that the Tauri app's
//! tauri-plugin-store maintains (see waypoints/src-tauri/src/registry_store.rs +
//! settings.rs), under the same `territories` key. That file lives at the root
//! of the flavor's app-data dir (e.g. `~/Library/Application Support/
//! so.waypoints.dev/settings.json`). Sharing it — rather than keeping a separate
//! registry — is what lets `waypoints-serve` see and open the territories the
//! desktop app already created.
//!
//! Note: this is a plain read-modify-write of the JSON file. If the desktop app
//! is running against the same flavor at the same time, its in-memory store can
//! clobber writes from here on its next save. Fine for dev; don't run both on
//! the same identifier concurrently.

use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::{Map, Value};

use cartograph::{RegistryStore, TerritoryError, TerritoryRegistry, TerritoryResult};

/// Key within `settings.json` holding the territory registry. Mirrors
/// `settings::KEY_TERRITORIES` in the Tauri crate.
const TERRITORIES_KEY: &str = "territories";

pub struct SettingsRegistryStore {
    /// Path to the shared `settings.json`.
    path: PathBuf,
}

impl SettingsRegistryStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn config_err(e: impl std::fmt::Display) -> TerritoryError {
        TerritoryError::Config(e.to_string())
    }

    /// Read `settings.json` into a JSON object, or an empty object if absent.
    async fn read_settings(&self) -> TerritoryResult<Map<String, Value>> {
        match tokio::fs::read(&self.path).await {
            Ok(bytes) => match serde_json::from_slice::<Value>(&bytes).map_err(Self::config_err)? {
                Value::Object(map) => Ok(map),
                _ => Err(TerritoryError::Config(format!(
                    "{} is not a JSON object",
                    self.path.display()
                ))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
            Err(e) => Err(Self::config_err(e)),
        }
    }
}

#[async_trait]
impl RegistryStore for SettingsRegistryStore {
    async fn load(&self) -> TerritoryResult<TerritoryRegistry> {
        let settings = self.read_settings().await?;
        match settings.get(TERRITORIES_KEY) {
            // Missing key is normal on first run — start from defaults.
            None => Ok(TerritoryRegistry::default()),
            Some(value) => serde_json::from_value(value.clone()).map_err(Self::config_err),
        }
    }

    async fn save(&self, registry: &TerritoryRegistry) -> TerritoryResult<()> {
        // Read-modify-write so we preserve any other keys plugin-store owns.
        let mut settings = self.read_settings().await?;
        settings.insert(
            TERRITORIES_KEY.to_string(),
            serde_json::to_value(registry).map_err(Self::config_err)?,
        );

        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(Self::config_err)?;
        }
        let bytes =
            serde_json::to_vec_pretty(&Value::Object(settings)).map_err(Self::config_err)?;
        tokio::fs::write(&self.path, bytes)
            .await
            .map_err(Self::config_err)
    }
}
