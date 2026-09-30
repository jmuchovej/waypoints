#![allow(clippy::disallowed_macros)]

use anyhow::{Context as _, Result};
use clap::Parser;
use ed25519_dalek::SigningKey;
use outpost::station::Station;
use std::path::{Path, PathBuf};

/// Run a Station over QUIC. It homes whatever Territories Vessels register
/// to it, persisting them under `<data>/territories/` and reloading them
/// on restart.
#[derive(Parser)]
pub struct StationArgs {
    /// Data directory for the Station's key, address, and Territory holds.
    #[arg(long, default_value = ".outpost-station")]
    data: PathBuf,
    /// Also run the per-Fleet orderer with a fresh fleet key (dev).
    #[arg(long)]
    orderer: bool,
    /// Socket address(es) to advertise to Vessels (e.g. `203.0.113.7:7331`)
    /// — the operator's public host:port. Repeatable. Binds routable and
    /// publishes these instead of the bound sockets.
    #[arg(long)]
    advertise: Vec<std::net::SocketAddr>,
}

pub async fn run(args: StationArgs) -> Result<()> {
    let StationArgs {
        data,
        orderer,
        advertise,
    } = args;
    std::fs::create_dir_all(data.as_path())?;
    let secret = station_key(&data)?;
    let territories = data.join("territories");

    // Self-hosting means other hosts must reach us: bind routable and
    // advertise the operator-supplied address.
    let station = if orderer {
        // Dev: a fresh fleet key co-located with the station.
        let mut rng = rand::rng();
        Station::hosting_orderer(&secret, SigningKey::generate(&mut rng), &territories).await?
    } else {
        Station::hosting_routable(&secret, &territories).await?
    }
    .with_advertised(advertise);

    // Publish the dialable address for vessels.
    let addr = station.advertised_addr();
    std::fs::write(station_addr_path(&data), serde_json::to_vec(&addr)?)?;

    println!("Station running (Ctrl-C to stop)");
    println!("  territories under {}", territories.display());
    println!(
        "  address published to {}",
        station_addr_path(&data).display()
    );
    if orderer {
        println!("  orderer: on");
    }

    // Serve concurrently (live subscriptions park sessions) until
    // interrupted.
    let station = std::sync::Arc::new(station);
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\nshutting down");
            Ok(())
        }
        r = station.serve_loop() => r.map_err(Into::into),
    }
}

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
