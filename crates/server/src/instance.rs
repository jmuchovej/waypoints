//! Per-flavor runtime resolution.
//!
//! The flavor (`so.waypoints.dev` / `.preview` / `so.waypoints`) is a logical
//! token the client supplies via the `X-Waypoints-Flavor` header — not a path.
//! The server resolves it to `<data_root>/<flavor>` (the same app-data dir the
//! matching desktop build uses), lazily builds a `WaypointsRuntime` for it, and
//! caches it. So the server knows *how* Waypoints works but stays agnostic about
//! *which* instance(s) it serves — it serves whatever flavor connects.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use tokio::sync::Mutex;

use cartograph::WaypointsRuntime;

use crate::registry::SettingsRegistryStore;

const FLAVOR_HEADER: &str = "x-waypoints-flavor";
const DEFAULT_FLAVOR: &str = "so.waypoints.dev";

/// Lazily-built, cached `WaypointsRuntime` per flavor identifier.
pub struct Instances {
    data_root: PathBuf,
    runtimes: Mutex<HashMap<String, Arc<WaypointsRuntime>>>,
}

impl Instances {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            runtimes: Mutex::new(HashMap::new()),
        }
    }

    /// Get (or build + resume + cache) the runtime for `flavor`.
    async fn get(&self, flavor: &str) -> Arc<WaypointsRuntime> {
        let mut runtimes = self.runtimes.lock().await;
        if let Some(rt) = runtimes.get(flavor) {
            return rt.clone();
        }

        let base = self.data_root.join(flavor);
        tracing::info!(flavor, base = %base.display(), "initializing runtime for flavor");

        // Share the desktop app's settings.json so we see its territories.
        let registry = Arc::new(SettingsRegistryStore::new(base.join("settings.json")));
        let rt = Arc::new(WaypointsRuntime::new(base.join("territories"), registry));
        if let Err(e) = rt.resume().await {
            tracing::warn!("failed to resume territory for {flavor}: {e}");
        }

        runtimes.insert(flavor.to_string(), rt.clone());
        rt
    }
}

pub type AppState = Arc<Instances>;

/// Extractor yielding the `WaypointsRuntime` for the request's flavor
/// (`X-Waypoints-Flavor`, defaulting to the dev flavor when absent).
pub struct Instance(pub Arc<WaypointsRuntime>);

impl FromRequestParts<AppState> for Instance {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let flavor = parts
            .headers
            .get(FLAVOR_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(DEFAULT_FLAVOR);

        if !valid_flavor(flavor) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("invalid {FLAVOR_HEADER}: {flavor:?}"),
            )
                .into_response());
        }

        Ok(Instance(state.get(flavor).await))
    }
}

/// Reject anything that could escape `data_root` when joined as a path component
/// (slashes, `..`, control chars). Flavors look like `so.waypoints[.<word>]`.
fn valid_flavor(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && !s.contains("..")
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}
