//! The `outpost` operator CLI — the two things that genuinely run without an
//! app around them: a Station process, and a forcible reconcile.
//!
//! Everything a *user* does — minting an identity, creating a Territory — is
//! outpost's library API
//! ([`KeyRoot`](outpost::protocol::KeyRoot),
//! [`Vessel::create_territory`](outpost::vessel::Vessel::create_territory)),
//! which the app (Waypoints, cartograph) drives through its UI. Outpost offers
//! the capability; the app orchestrates it — outpost never needs to know how
//! to *use* Waypoints, only how to serve it. So this binary is
//! deliberately thin:
//!
//! - `station-run`: run a Station serving a Territory over QUIC (optionally as
//!   the fleet orderer). A server process — no app wraps it.
//! - `sync`: force an out-of-band reconcile of an app-managed data directory
//!   with a Station. Not the normal path — devices reconcile on demand as the
//!   app opens a connection; this is the manual escape hatch.
//!
//! First-cut limits (noted inline): one Territory per Station, plaintext key
//! custody, a fresh dev fleet key under `--orderer`, and a file-published
//! ephemeral address.

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use tokio::runtime::Runtime as TokioRuntime;

mod commands {
    pub(crate) mod station;
    pub(crate) mod sync;
}
pub(crate) mod utils;

#[derive(Parser)]
#[command(name = "outpost", about = "Waypoints sync — operator CLI")]
#[allow(clippy::upper_case_acronyms)]
struct CLI {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Station(commands::station::StationArgs),
    Sync(commands::sync::SyncArgs),
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "outpost=info".into()),
        )
        .init();

    let cli = CLI::parse();
    let rt = TokioRuntime::new()?;
    rt.block_on(dispatch(cli.command))
}

async fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Station(args) => commands::station::run(args).await,
        Command::Sync(args) => commands::sync::run(args).await,
    }
}
