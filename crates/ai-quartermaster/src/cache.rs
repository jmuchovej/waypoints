//! Cache location for downloaded model artifacts. Layout inside the
//! directory is hf-hub's cache format (revision-addressed snapshots), which
//! gives us dedup and integrity for free.

use std::path::PathBuf;

/// Default artifact directory when the embedding app doesn't supply one:
/// the platform app-data dir under the `so.waypoints` identifier (matching
/// the Tauri bundle id), plus `models/`. The Tauri layer should prefer
/// passing its own resolved app-data path so flavors (dev/preview/stable)
/// keep separate caches.
pub fn default_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("so", "", "waypoints").map(|dirs| dirs.data_dir().join("models"))
}
