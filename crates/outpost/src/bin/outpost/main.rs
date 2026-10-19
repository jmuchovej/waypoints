//! The `outpost` operator CLI — what genuinely runs without an app around
//! it: a Station process, a forcible reconcile, and the key files the
//! Station is handed.
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
//! - `station`: run a Station over QUIC — storage always, the fleet orderer
//!   when handed a fleet key — homing every Territory Vessels register to
//!   it. A server process, built to run under a supervisor: a fixed port
//!   shared by QUIC (UDP) and the probe routes (TCP), keys from files, a
//!   clean stop on SIGTERM.
//! - `sync`: force an out-of-band reconcile of an app-managed data directory
//!   with a Station. Not the normal path — devices reconcile on demand as the
//!   app opens a connection; this is the manual escape hatch.
//! - `keygen`: write a fresh key file for `station --key-file` or
//!   `--fleet-key-file`, so a Secret can exist before the first start.
//!
//! Key custody is plaintext files: an operator's Secret mount, or the data
//! directory in development. The Station never generates a key at a path it
//! was handed.

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tokio::runtime::Runtime as TokioRuntime;

mod commands {
    pub(crate) mod keygen;
    pub(crate) mod station;
    pub(crate) mod sync;
}
pub(crate) mod keys;
pub(crate) mod probes;
pub(crate) mod utils;

#[derive(Parser)]
#[command(name = "outpost", about = "Waypoints sync — operator CLI")]
#[allow(clippy::upper_case_acronyms)]
struct CLI {
    /// Log format: human-readable text, or one JSON object per line
    /// (`timestamp`, `level`, `fields.message`, …) for log collectors.
    #[arg(
        long,
        global = true,
        env = "OUTPOST_LOG_FORMAT",
        value_enum,
        default_value_t = LogFormat::Text
    )]
    log_format: LogFormat,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum LogFormat {
    #[default]
    Text,
    Json,
}

#[derive(Subcommand)]
enum Command {
    Station(commands::station::StationArgs),
    Sync(commands::sync::SyncArgs),
    Keygen(commands::keygen::KeygenArgs),
}

fn main() -> Result<()> {
    // Parse before installing logging: the format is an argument.
    let cli = CLI::parse();
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "outpost=info".into());
    match cli.log_format {
        LogFormat::Text => tracing_subscriber::fmt().with_env_filter(filter).init(),
        LogFormat::Json => tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init(),
    }

    let rt = TokioRuntime::new()?;
    rt.block_on(dispatch(cli.command))
}

async fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Station(args) => commands::station::run(args).await,
        Command::Sync(args) => commands::sync::run(args).await,
        Command::Keygen(args) => commands::keygen::run(args),
    }
}
