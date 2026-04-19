//! The §4 endpoints as an axum router. Deliberately dumb: stores
//! envelope payloads byte-verbatim, assigns per-realm arrival seqs, and
//! never parses `checkpoints`.

use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::auth::{AuthError, Identity, Validator};

use super::service::{UploadRejection, accept_upload, serve_fetch};
use super::store::EnvelopeStore;
use super::wire::{DEFAULT_LIMIT, MAX_BATCH, MAX_BODY_BYTES, MAX_LIMIT};

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn EnvelopeStore>,
    pub validator: Arc<Validator>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/realms/{realm_id}/envelopes", get(fetch).post(upload))
        .route("/healthz", get(async || "ok"))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn problem(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(ErrorBody {
            error: message.into(),
        }),
    )
        .into_response()
}

fn bearer(headers: &HeaderMap) -> &str {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
}

async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    realm_id: &str,
) -> Result<Identity, Response> {
    state
        .validator
        .validate(bearer(headers), realm_id)
        .await
        .map_err(|e| match e {
            AuthError::Missing | AuthError::Unknown => {
                problem(StatusCode::UNAUTHORIZED, e.to_string())
            }
            AuthError::NoGrant => problem(StatusCode::FORBIDDEN, e.to_string()),
            AuthError::Upstream(_) => problem(StatusCode::BAD_GATEWAY, e.to_string()),
        })
}

// ── Upload ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct UploadBody<'a> {
    #[serde(borrow)]
    envelopes: Vec<&'a RawValue>,
}

async fn upload(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let identity = match authenticate(&state, &headers, &realm_id).await {
        Ok(i) => i,
        Err(response) => return response,
    };

    let parsed: UploadBody = match serde_json::from_str(&body) {
        Ok(b) => b,
        Err(e) => return problem(StatusCode::BAD_REQUEST, format!("malformed body: {e}")),
    };
    if parsed.envelopes.len() > MAX_BATCH {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("batch cap is {MAX_BATCH} envelopes"),
        );
    }

    let response = accept_upload(
        state.storage.as_ref(),
        &realm_id,
        &parsed.envelopes,
        identity.account_id.as_deref(),
    )
    .await;
    match response {
        Ok(body) => Json(body).into_response(),
        Err(UploadRejection::Malformed(message)) => problem(StatusCode::BAD_REQUEST, message),
        Err(UploadRejection::RealmMismatch { group_id }) => problem(
            StatusCode::BAD_REQUEST,
            format!("envelope {group_id} realm mismatch"),
        ),
        Err(UploadRejection::AccountMismatch { group_id }) => problem(
            StatusCode::FORBIDDEN,
            format!("envelope {group_id} account mismatch"),
        ),
        Err(UploadRejection::Store(e)) => {
            tracing::error!(error = %e, "append failed");
            problem(StatusCode::INTERNAL_SERVER_ERROR, "storage failure")
        }
    }
}

// ── Fetch ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct FetchParams {
    #[serde(default)]
    since_seq: i64,
    #[serde(default)]
    exclude_device: String,
    limit: Option<usize>,
}

async fn fetch(
    State(state): State<AppState>,
    Path(realm_id): Path<String>,
    Query(params): Query<FetchParams>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = authenticate(&state, &headers, &realm_id).await {
        return response;
    }
    if params.since_seq < 0 {
        return problem(StatusCode::BAD_REQUEST, "since_seq must be non-negative");
    }
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let page = serve_fetch(
        state.storage.as_ref(),
        &realm_id,
        params.since_seq,
        &params.exclude_device,
        limit,
    )
    .await;
    match page {
        Ok(body) => Json(body).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "page failed");
            problem(StatusCode::INTERNAL_SERVER_ERROR, "storage failure")
        }
    }
}
