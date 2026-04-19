//! `waypoints-serve` — an HTTP backend for the Waypoints web UI.
//!
//! It wraps the same `cartograph::WaypointsRuntime` the Tauri app uses and
//! exposes it over explicit per-command routes (see [`commands`]) at
//! `/ipc/<command_name>`, so the Nuxt frontend can run in a plain browser
//! without Tauri. The client picks which instance via the `X-Waypoints-Flavor`
//! header (see [`instance`]) — the server resolves `<data_root>/<flavor>`, so it
//! bakes in no flavor of its own. Currently a development convenience; not
//! feature-gated.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use clap::Parser;
use directories::BaseDirs;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::instance::{AppState, Instances};

mod commands;
mod error;
mod instance;
mod registry;

#[derive(Parser, Debug)]
#[command(
    name = "waypoints-serve",
    about = "HTTP backend for the Waypoints web UI"
)]
struct Cli {
    /// Port to listen on (localhost only).
    #[arg(long, default_value_t = 8123)]
    port: u16,

    /// Root the per-request flavor resolves under: a flavor `F` opens
    /// `<data-root>/F/{settings.json,territories/}`. Defaults to the platform
    /// app-data dir (matching Tauri's `app_data_dir()` parent), so flavors land
    /// on the same dirs the desktop builds use. Mainly an override for tests.
    #[arg(long)]
    data_root: Option<PathBuf>,

    /// Directory of the built Nuxt SPA to serve (e.g.
    /// `waypoints/.output/public`). When omitted, only `/ipc` is served and the
    /// frontend is expected to come from the Nuxt dev server via proxy.
    #[arg(long)]
    static_dir: Option<PathBuf>,
}

/// Platform app-data dir (the parent that `<flavor>` dirs sit under), matching
/// the parent of Tauri's `app_data_dir()`.
fn default_data_root() -> PathBuf {
    BaseDirs::new()
        .map(|b| b.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".waypoints-serve"))
}

async fn serve(cli: Cli) {
    let data_root = cli.data_root.unwrap_or_else(default_data_root);
    tracing::info!(
        data_root = %data_root.display(),
        "ready; instance selected per-request via X-Waypoints-Flavor"
    );

    // Runtimes are built lazily per flavor on first request — nothing to resume
    // up front.
    let instances: AppState = Arc::new(Instances::new(data_root));

    let mut router: Router<AppState> = commands::router();

    if let Some(static_dir) = cli.static_dir {
        // SPA fallback: unknown paths serve index.html so client-side routing
        // works.
        let index = static_dir.join("index.html");
        let serve_dir = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index));
        router = router.fallback_service(serve_dir);
        tracing::info!(path = %static_dir.display(), "serving static SPA");
    }

    let app = router
        .with_state(instances)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {addr}: {e}"));
    tracing::info!("waypoints-serve listening on http://{addr}");

    axum::serve(listener, app).await.expect("server error");
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    runtime.block_on(serve(cli));
}
