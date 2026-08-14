#![allow(clippy::disallowed_macros)]

use crate::utils::{load_identity, territory_db};
use anyhow::{Context as _, Result};
use clap::Parser;
use outpost::{fabric::EndpointAddr, hold::TerritoryHold, replica::Replica, vessel::Vessel};
use std::path::PathBuf;

/// Force a reconcile of an app-managed data directory with a Station.
#[derive(Parser)]
pub struct SyncArgs {
    /// Data directory the app created (holds the mnemonic + Territory).
    #[arg(long, default_value = ".outpost")]
    data: PathBuf,
    /// The Station's address file (`station.addr` written by
    /// `station-run`).
    #[arg(long)]
    station: PathBuf,
}

pub async fn run(args: SyncArgs) -> Result<()> {
    let SyncArgs {
        data,
        station: station_addr_file,
    } = args;

    let (identity, device) = load_identity(&data)?;
    let addr: EndpointAddr = serde_json::from_slice(&std::fs::read(station_addr_file)?)?;

    // Reopen the existing Territory.
    let hold = TerritoryHold::open(territory_db(&data).to_str().context("non-utf8 path")?).await?;
    let replica = Replica::reopen(hold).await?;
    let mut vessel = Vessel::from_identity(&identity, &device, replica).await?;
    let territory = vessel.territory_id().await?;

    let before = vessel.replica().acl_len().await?;
    // Establish the sync chain: register the Territory (idempotent — the
    // Station starts serving it if it was not already), then reconcile.
    vessel.register(addr.clone(), territory).await?;
    vessel.sync_with(addr).await?;
    let after = vessel.replica().acl_len().await?;
    let notices = vessel.replica().hold().held_ids().await?.len();

    println!("Reconciled.");
    println!("  acl records: {before} -> {after}");
    println!("  notices held: {notices}");
    Ok(())
}
