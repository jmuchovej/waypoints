use std::sync::Arc;
use std::time::Duration;

use clap::Parser;

use outpost::auth::Validator;
use outpost::sync::peers::client::PeerClient;
use outpost::sync::peers::pump::pump_all;
use outpost::sync::peers::turso::Storage;
use outpost::{AppState, router};

/// The always-online peer (see PROTOCOL.md in this crate).
///
/// Every flag falls back to its `OUTPOST_*` environment variable, so
/// container/systemd/nix deployments stay env-only; secrets in
/// particular belong in the environment, out of `ps`.
#[derive(Parser)]
#[command(name = "outpost", version)]
struct Cli {
    /// Listen address.
    #[arg(long, env = "OUTPOST_BIND", default_value = "127.0.0.1:8600")]
    bind: String,

    /// Envelope database path.
    #[arg(long, env = "OUTPOST_DB", default_value = "./outpost.db")]
    db: String,

    /// Self-host auth: the one shared secret (PROTOCOL.md §6).
    #[arg(long, env = "OUTPOST_SHARED_SECRET", hide_env_values = true)]
    shared_secret: Option<String>,

    /// Official auth: accounts database URL (needs --accounts-db-token).
    #[arg(long, env = "OUTPOST_ACCOUNTS_DB_URL")]
    accounts_db_url: Option<String>,

    /// Official auth: accounts database token.
    #[arg(long, env = "OUTPOST_ACCOUNTS_DB_TOKEN", hide_env_values = true)]
    accounts_db_token: Option<String>,

    /// Peer base URL to gossip-pull from (repeat, or comma-separate in
    /// the env var). See PROTOCOL.md §1 "Gossip".
    #[arg(long = "peer", env = "OUTPOST_PEERS", value_delimiter = ',')]
    peers: Vec<String>,

    /// Bearer presented to gossip peers (required with --peer).
    #[arg(long, env = "OUTPOST_PEER_TOKEN", hide_env_values = true)]
    peer_token: Option<String>,

    /// Extra realm to gossip-pull (bootstrap for a fresh mirror;
    /// locally-known realms are always pulled).
    #[arg(
        long = "peer-realm",
        env = "OUTPOST_PEER_REALMS",
        value_delimiter = ','
    )]
    peer_realms: Vec<String>,

    /// Seconds between gossip rounds.
    #[arg(long, env = "OUTPOST_PEER_INTERVAL", default_value_t = 5)]
    peer_interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "outpost=info,tower_http=info".into()),
        )
        .init();

    let cli = Cli::parse();

    let validator = match (&cli.shared_secret, &cli.accounts_db_url) {
        (Some(secret), _) if !secret.is_empty() => Validator::SharedSecret(secret.clone()),
        (_, Some(url)) => Validator::AccountsDb {
            url: url.clone(),
            token: cli
                .accounts_db_token
                .clone()
                .ok_or("--accounts-db-token (OUTPOST_ACCOUNTS_DB_TOKEN) is required with --accounts-db-url")?,
            http: reqwest::Client::new(),
        },
        _ => {
            return Err("set --shared-secret or --accounts-db-url (+ token)".into());
        }
    };

    let storage = Arc::new(Storage::open(&cli.db).await?);
    let state = AppState {
        storage: storage.clone(),
        validator: Arc::new(validator),
    };

    if let Some((peers, extra_realms, interval)) = gossip_config(&cli)? {
        let store = storage.clone();
        tracing::info!(
            peers = peers.len(),
            interval_secs = interval.as_secs(),
            "gossip pump enabled"
        );
        tokio::spawn(async move {
            loop {
                let outcome = pump_all(store.as_ref(), &peers, &extra_realms).await;
                if outcome.fetched > 0 {
                    tracing::info!(
                        fetched = outcome.fetched,
                        stored = outcome.stored,
                        dropped = outcome.dropped,
                        "gossip round"
                    );
                }
                tokio::time::sleep(interval).await;
            }
        });
    }

    let listener = tokio::net::TcpListener::bind(&cli.bind).await?;
    tracing::info!(bind = %cli.bind, db = %cli.db, "outpost listening");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

type GossipConfig = (Vec<PeerClient>, Vec<String>, Duration);

fn gossip_config(cli: &Cli) -> Result<Option<GossipConfig>, Box<dyn std::error::Error>> {
    let urls: Vec<&str> = cli
        .peers
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if urls.is_empty() {
        return Ok(None);
    }

    let token = cli
        .peer_token
        .as_ref()
        .ok_or("--peer-token (OUTPOST_PEER_TOKEN) is required with --peer")?;
    let peers = urls
        .into_iter()
        .map(|url| PeerClient::new(url, token.clone()))
        .collect();

    let extra_realms = cli
        .peer_realms
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    Ok(Some((
        peers,
        extra_realms,
        Duration::from_secs(cli.peer_interval),
    )))
}
