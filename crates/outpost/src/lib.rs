//! Outpost — the always-online peer. See PROTOCOL.md in this crate.
//!
//! Deliberately dumb: stores envelope payloads byte-verbatim, assigns
//! per-realm arrival seqs, and never parses `checkpoints`. Conflict
//! resolution lives exclusively in the app's Rust core; an outpost that
//! tried to be clever would be wrong.

pub mod auth;
mod handlers;
pub mod storage;

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::get;

use crate::auth::Validator;
use crate::storage::Storage;

pub const MAX_BATCH: usize = 100;
pub const MAX_BODY_BYTES: usize = 1 << 20; // 1 MiB
pub const DEFAULT_LIMIT: usize = 200;
pub const MAX_LIMIT: usize = 500;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<Storage>,
    pub validator: Arc<Validator>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/realms/{realm_id}/envelopes",
            get(handlers::fetch).post(handlers::upload),
        )
        .route("/healthz", get(async || "ok"))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}
