use crate::{keys, probes};
use anyhow::{Context as _, Result};
use clap::Parser;
use ed25519_dalek::SigningKey;
use outpost::station::{Station, StationError};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// How long an orderly close may take before the process exits anyway —
/// well inside a supervisor's grace period, far shorter than a Vessel's
/// idle timeout.
const CLOSE_BOUND: Duration = Duration::from_secs(10);

/// How many kernel-chosen ports to try when `--port 0` lands on a number
/// that is free over TCP but taken over UDP.
const EPHEMERAL_ATTEMPTS: u8 = 8;

/// Run a Station over QUIC. It homes whatever Territories Vessels register
/// to it, persisting them under `<data>/territories/` and reloading them
/// on restart. Every option also reads from the `OUTPOST_*` variable named
/// beside it; a flag wins over the environment.
#[derive(Parser)]
pub struct StationArgs {
    /// Data directory: the address file, the Territory holds, and — when
    /// `--key-file` is not given — the identity key.
    #[arg(long, env = "OUTPOST_DATA", default_value = ".outpost-station")]
    data: PathBuf,
    /// Identity key file: 32 raw bytes, an Ed25519 seed (`outpost keygen`).
    /// Must exist — a Secret mount is read-only, and a key is never invented
    /// at a path the operator named. Without it, `<data>/station.key` is
    /// loaded, or generated on first start.
    #[arg(long, env = "OUTPOST_KEY_FILE")]
    key_file: Option<PathBuf>,
    /// Fleet signing key file, same format. Supplying it turns the orderer
    /// role on; the key is never generated here.
    #[arg(long, env = "OUTPOST_FLEET_KEY_FILE")]
    fleet_key_file: Option<PathBuf>,
    /// The Station's port: QUIC over UDP and the probe routes (`/healthz`,
    /// `/readyz`) over TCP share it. 0 lets the kernel choose a number that
    /// is free for both.
    #[arg(long, env = "OUTPOST_PORT", default_value_t = 6782)]
    port: u16,
    /// Socket address(es) to advertise to Vessels (e.g. `203.0.113.7:6782`)
    /// — the operator's public host:port, published instead of the bound
    /// sockets. Repeatable, or comma-separated in `OUTPOST_ADVERTISE`; an
    /// empty value advertises nothing.
    #[arg(long, env = "OUTPOST_ADVERTISE", value_delimiter = ',')]
    advertise: Vec<String>,
}

pub async fn run(args: StationArgs) -> Result<()> {
    let StationArgs {
        data,
        key_file,
        fleet_key_file,
        port,
        advertise,
    } = args;
    let advertise = parse_advertise(&advertise)?;
    std::fs::create_dir_all(&data)?;
    let secret = match &key_file {
        Some(path) => keys::load(path)?,
        None => keys::load_or_generate(&data.join("station.key"))?,
    };
    let fleet_key = fleet_key_file.as_deref().map(keys::load).transpose()?;
    let territories = data.join("territories");

    // Probes first: the TCP bind picks the number both sockets share, and
    // the routes are served before the storage scan, so a slow start reads
    // as "not ready" rather than "connection refused" or a hang.
    let readiness = probes::Readiness::default();
    let (bound, station) =
        bind_pair(port, &secret, fleet_key.as_ref(), &territories, &readiness).await?;
    let station = Arc::new(station.with_advertised(advertise));

    // Publish the dialable address for Vessels: the file, and the log —
    // the operator's copy when the data directory is a volume they cannot
    // read.
    let addr_json = serde_json::to_string(&station.advertised_addr())?;
    std::fs::write(data.join("station.addr"), &addr_json)?;

    let serving = Arc::clone(&station);
    let serve_loop = tokio::spawn(async move { serving.serve_loop().await });
    readiness.mark_ready();

    tracing::info!(
        port = bound,
        address = %addr_json,
        data = %data.display(),
        orderer = fleet_key.is_some(),
        "station running"
    );

    tokio::select! {
        _ = shutdown_signal() => {
            tracing::info!("termination signal received; closing the endpoint");
            // Awaiting the close is what lets peers see a close instead of
            // a timeout. A second signal, or the bound, ends the wait.
            tokio::select! {
                closed = tokio::time::timeout(CLOSE_BOUND, station.endpoint().close()) => {
                    if closed.is_err() {
                        tracing::warn!(bound = ?CLOSE_BOUND, "endpoint close timed out; exiting anyway");
                    }
                }
                _ = shutdown_signal() => {
                    tracing::warn!("second signal; exiting without waiting for the close");
                }
            }
            Ok(())
        }
        served = serve_loop => served.context("serve loop task failed")?.map_err(Into::into),
    }
}

/// Bind the probe listener and the Station on one port number, and start
/// serving the probes before the Station's constructor runs its storage
/// scan — a bound-but-unserved listener would leave a probe hanging in
/// the backlog for as long as the scan takes. TCP goes first because its
/// port can be read back; the Station then binds the same number. On a
/// configured port either failure is final. On port 0, a UDP collision
/// with the kernel's TCP pick is retried on a fresh pick, a bounded number
/// of times — the operator asked for any port, so a different one betrays
/// nothing. Returns the port and the Station; the probe server runs for
/// the life of the process.
async fn bind_pair(
    port: u16,
    secret: &SigningKey,
    fleet_key: Option<&SigningKey>,
    territories: &Path,
    readiness: &probes::Readiness,
) -> Result<(u16, Station)> {
    let mut attempt = 0u8;
    loop {
        let listener = probes::bind(port)
            .await
            .with_context(|| format!("binding TCP port {port} for the probe routes"))?;
        let bound = listener.local_addr()?.port();
        let server = tokio::spawn(probes::serve(listener, readiness.clone()));
        let built = match fleet_key {
            Some(fleet_key) => {
                Station::hosting_orderer_routable_at(secret, fleet_key.clone(), territories, bound)
                    .await
            }
            None => Station::hosting_routable_at(secret, territories, bound).await,
        };
        match built {
            Ok(station) => return Ok((bound, station)),
            Err(StationError::Transport(e)) if port == 0 && attempt < EPHEMERAL_ATTEMPTS => {
                attempt += 1;
                tracing::debug!(
                    port = bound,
                    attempt,
                    error = %e,
                    "kernel-chosen port is taken over UDP; trying another"
                );
                // Aborting the server drops its listener, freeing the port.
                server.abort();
            }
            Err(e) => {
                server.abort();
                return Err(e).with_context(|| format!("binding UDP port {bound} for QUIC"));
            }
        }
    }
}

/// Resolves on SIGTERM or SIGINT (Ctrl-C) — the two ways a supervisor or
/// a terminal asks a process to stop. Off Unix only Ctrl-C exists.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = terminate.recv() => {}
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "cannot listen for SIGTERM; Ctrl-C only");
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// `--advertise` values, each a `host:port` socket. Empty entries are
/// dropped, so an empty `OUTPOST_ADVERTISE` (the manifests' default) means
/// "advertise nothing"; a non-empty entry that is not a socket address is
/// an error naming it.
fn parse_advertise(values: &[String]) -> Result<Vec<SocketAddr>> {
    values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.parse::<SocketAddr>()
                .with_context(|| format!("--advertise {v:?} is not a socket address (host:port)"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    #[test]
    fn an_empty_advertise_means_none() {
        assert!(parse_advertise(&[]).unwrap().is_empty());
        assert!(parse_advertise(&strings(&[""])).unwrap().is_empty());
        assert!(parse_advertise(&strings(&["", " "])).unwrap().is_empty());
    }

    #[test]
    fn a_bad_entry_is_named() {
        let err = parse_advertise(&strings(&["a:1", "b:2"]))
            .unwrap_err()
            .to_string();
        assert!(err.contains("\"a:1\""), "{err}");
    }

    #[test]
    fn sockets_of_both_families_parse() {
        let sockets =
            parse_advertise(&strings(&["203.0.113.7:6782", "[2001:db8::1]:6782"])).unwrap();
        assert_eq!(sockets.len(), 2);
        assert!(sockets.iter().all(|s| s.port() == 6782));
        assert!(sockets[0].is_ipv4() && sockets[1].is_ipv6());
    }
}
