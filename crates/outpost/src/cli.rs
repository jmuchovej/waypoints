//! The `outpost` operator CLI — the two things that genuinely run without an
//! app around them: a Station process, and a forcible reconcile.
//!
//! Everything a *user* does — minting an identity, creating a Territory — is
//! outpost's library API
//! ([`KeyRoot`](outpost_protocol::identity::KeyRoot),
//! [`Vessel::create_territory`](crate::vessel::Vessel::create_territory)),
//! which the app (Waypoints, cartograph) drives through its UI. Outpost offers
//! the capability; the app orchestrates it — outpost never needs to know how
//! to *use* Waypoints, only how to serve it (§12/§13). So this binary is
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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use outpost_hold::TerritoryHold;
use outpost_protocol::acl::ACLRecord;
use outpost_protocol::identity::{DeviceKey, Identity, KeyRoot};

use crate::replica::Replica;
use crate::station::Station;
use crate::vessel::Vessel;

#[derive(Parser)]
#[command(name = "outpost", about = "Waypoints sync — operator CLI")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a Station serving one Territory over QUIC.
    StationRun {
        /// The Territory's exported root record — a dev bootstrap. The app
        /// exports this via the library API; registration-driven serving is
        /// the follow-up.
        #[arg(long)]
        root: PathBuf,
        /// Data directory for the Station's storage + key + address.
        #[arg(long, default_value = ".outpost-station")]
        data: PathBuf,
        /// Also run the per-Fleet orderer with a fresh fleet key (dev).
        #[arg(long)]
        orderer: bool,
    },
    /// Force a reconcile of an app-managed data directory with a Station.
    Sync {
        /// Data directory the app created (holds the mnemonic + Territory).
        #[arg(long, default_value = ".outpost")]
        data: PathBuf,
        /// The Station's address file (`station.addr` written by
        /// `station-run`).
        #[arg(long)]
        station: PathBuf,
    },
}

/// Binary entry point.
pub fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "outpost=info".into()),
        )
        .init();
    let cli = Cli::parse();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run(cli.command))
}

async fn run(command: Command) -> Result<()> {
    match command {
        Command::StationRun {
            root,
            data,
            orderer,
        } => station_run(&root, &data, orderer).await,
        Command::Sync { data, station } => sync(&data, &station).await,
    }
}

// ---- shared: reading an app-managed data directory ------------------------

fn mnemonic_path(data: &Path) -> PathBuf {
    data.join("mnemonic.txt")
}

fn territory_db(data: &Path) -> PathBuf {
    data.join("territory.db")
}

/// Load the identity + first device key from the data directory's mnemonic.
/// The app writes the mnemonic; this only reads it.
fn load_identity(data: &Path) -> Result<(Identity, DeviceKey)> {
    let phrase = std::fs::read_to_string(mnemonic_path(data)).with_context(|| {
        format!(
            "no identity in {} (expected a mnemonic written by the app)",
            data.display()
        )
    })?;
    let root = KeyRoot::from_mnemonic(phrase.trim())?;
    Ok((root.identity(), root.device(0)))
}

// ---- station --------------------------------------------------------------

fn station_key_path(data: &Path) -> PathBuf {
    data.join("station.key")
}

fn station_addr_path(data: &Path) -> PathBuf {
    data.join("station.addr")
}

/// Load or generate the Station's identity key (dev: plaintext on disk).
fn station_key(data: &Path) -> Result<SigningKey> {
    let path = station_key_path(data);
    if let Ok(bytes) = std::fs::read(&path) {
        let bytes: [u8; 32] = bytes.as_slice().try_into().context("bad station key")?;
        Ok(SigningKey::from_bytes(&bytes))
    } else {
        let mut rng = rand::rng();
        let key = SigningKey::generate(&mut rng);
        std::fs::write(&path, key.to_bytes())?;
        Ok(key)
    }
}

async fn station_run(root: &Path, data: &Path, orderer: bool) -> Result<()> {
    std::fs::create_dir_all(data)?;
    let root_record = ACLRecord::decode(&std::fs::read(root)?)?;

    // Open the Station's hold; bootstrap it from the root on first run.
    let hold = TerritoryHold::open(station_db(data)?.as_str()).await?;
    if hold.acl_records().await?.is_empty() {
        hold.append_acl(&root_record, 0, None).await?;
    }
    let replica = Replica::reopen(hold).await?;

    let secret = station_key(data)?;
    let station = if orderer {
        // Dev: a fresh fleet key co-located with the station.
        let mut rng = rand::rng();
        Station::orderer(&secret, SigningKey::generate(&mut rng), replica).await?
    } else {
        Station::new(&secret, replica).await?
    };

    // Publish the dialable address for vessels.
    let addr = station.endpoint().direct_addr();
    std::fs::write(station_addr_path(data), serde_json::to_vec(&addr)?)?;

    println!("Station running (Ctrl-C to stop)");
    println!(
        "  address published to {}",
        station_addr_path(data).display()
    );
    if orderer {
        println!("  orderer: on");
    }

    // Serve until interrupted.
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nshutting down");
                return Ok(());
            }
            r = station.serve_once() => {
                if let Err(e) = r {
                    tracing::warn!("serve error: {e}");
                }
            }
        }
    }
}

fn station_db(data: &Path) -> Result<String> {
    Ok(data
        .join("station.db")
        .to_str()
        .context("non-utf8 path")?
        .to_owned())
}

// ---- sync -----------------------------------------------------------------

async fn sync(data: &Path, station_addr_file: &Path) -> Result<()> {
    let (identity, device) = load_identity(data)?;
    let addr: outpost_fabric::EndpointAddr =
        serde_json::from_slice(&std::fs::read(station_addr_file)?)?;

    // Reopen the existing Territory.
    let hold = TerritoryHold::open(territory_db(data).to_str().context("non-utf8 path")?).await?;
    let replica = Replica::reopen(hold).await?;
    let mut vessel = Vessel::from_identity(&identity, &device, replica).await?;

    let before = vessel.replica().acl_len().await?;
    vessel.sync_with(addr).await?;
    let after = vessel.replica().acl_len().await?;
    let notices = vessel.replica().hold().held_ids().await?.len();

    println!("Reconciled.");
    println!("  acl records: {before} -> {after}");
    println!("  notices held: {notices}");
    Ok(())
}
