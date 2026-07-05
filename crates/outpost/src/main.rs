use std::sync::Arc;

use outpost::auth::Validator;
use outpost::storage::Storage;
use outpost::{AppState, router};

/// Configuration is environment-only (12-factor; container/systemd/nix
/// friendly):
///
/// - `OUTPOST_BIND`          listen address     (default `127.0.0.1:8600`)
/// - `OUTPOST_DB`            envelope db path   (default `./outpost.db`)
/// - auth, one of:
///   - `OUTPOST_SHARED_SECRET`                       self-host mode
///   - `OUTPOST_ACCOUNTS_DB_URL` + `_TOKEN`          official mode
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "outpost=info,tower_http=info".into()),
        )
        .init();

    let bind = std::env::var("OUTPOST_BIND").unwrap_or_else(|_| "127.0.0.1:8600".into());
    let db_path = std::env::var("OUTPOST_DB").unwrap_or_else(|_| "./outpost.db".into());

    let validator = match (
        std::env::var("OUTPOST_SHARED_SECRET"),
        std::env::var("OUTPOST_ACCOUNTS_DB_URL"),
    ) {
        (Ok(secret), _) if !secret.is_empty() => Validator::SharedSecret(secret),
        (_, Ok(url)) => Validator::AccountsDb {
            url,
            token: std::env::var("OUTPOST_ACCOUNTS_DB_TOKEN")?,
            http: reqwest::Client::new(),
        },
        _ => {
            return Err(
                "set OUTPOST_SHARED_SECRET or OUTPOST_ACCOUNTS_DB_URL (+_TOKEN)".into(),
            );
        }
    };

    let storage = Storage::open(&db_path).await?;
    let state = AppState {
        storage: Arc::new(storage),
        validator: Arc::new(validator),
    };

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, %db_path, "outpost listening");
    axum::serve(listener, router(state)).await?;
    Ok(())
}
