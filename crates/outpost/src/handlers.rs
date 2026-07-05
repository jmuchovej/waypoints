use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::auth::{AuthError, Identity};
use crate::{AppState, DEFAULT_LIMIT, MAX_BATCH, MAX_LIMIT};

/// The routing fields the outpost reads. Everything else in an envelope —
/// `checkpoints` above all — passes through byte-verbatim.
#[derive(Debug, Deserialize)]
pub struct EnvelopeHead {
    pub group_id: String,
    pub account_id: String,
    pub realm_id: String,
    pub device_id: String,
    pub hlc: String,
    #[serde(skip)]
    pub hlc_i64: i64,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn problem(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(ErrorBody { error: message.into() })).into_response()
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
            AuthError::Missing | AuthError::Unknown => problem(StatusCode::UNAUTHORIZED, e.to_string()),
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

#[derive(Serialize)]
struct UploadAck {
    group_id: String,
    seq: i64,
}

#[derive(Serialize)]
struct UploadResponse {
    results: Vec<UploadAck>,
    max_seq: i64,
}

pub async fn upload(
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

    let mut heads = Vec::with_capacity(parsed.envelopes.len());
    for raw in &parsed.envelopes {
        let mut head: EnvelopeHead = match serde_json::from_str(raw.get()) {
            Ok(h) => h,
            Err(e) => return problem(StatusCode::BAD_REQUEST, format!("malformed envelope: {e}")),
        };
        if head.realm_id != realm_id {
            return problem(
                StatusCode::BAD_REQUEST,
                format!("envelope {} realm mismatch", head.group_id),
            );
        }
        if let Some(account) = &identity.account_id {
            if &head.account_id != account {
                return problem(
                    StatusCode::FORBIDDEN,
                    format!("envelope {} account mismatch", head.group_id),
                );
            }
        }
        head.hlc_i64 = match head.hlc.parse::<i64>() {
            Ok(v) if v >= 0 => v,
            _ => {
                return problem(
                    StatusCode::BAD_REQUEST,
                    format!("envelope {} hlc must be a decimal string", head.group_id),
                );
            }
        };
        heads.push(head);
    }

    let payloads: Vec<&str> = parsed.envelopes.iter().map(|r| r.get()).collect();
    let received_at = chrono_free_timestamp();
    let acks = match state.storage.append(&heads, &payloads, &received_at).await {
        Ok(acks) => acks,
        Err(e) => {
            tracing::error!(error = %e, "append failed");
            return problem(StatusCode::INTERNAL_SERVER_ERROR, "storage failure");
        }
    };

    let results: Vec<UploadAck> = acks
        .into_iter()
        .map(|(group_id, seq)| UploadAck { group_id, seq })
        .collect();
    let max_seq = results.iter().map(|r| r.seq).max().unwrap_or(0);
    Json(UploadResponse { results, max_seq }).into_response()
}

// ── Fetch ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FetchParams {
    #[serde(default)]
    since_seq: i64,
    #[serde(default)]
    exclude_device: String,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct SequencedEnvelope {
    seq: i64,
    envelope: Box<RawValue>,
}

#[derive(Serialize)]
struct FetchResponse {
    envelopes: Vec<SequencedEnvelope>,
    next_since: i64,
    has_more: bool,
}

pub async fn fetch(
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

    let window = match state.storage.page(&realm_id, params.since_seq, limit).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "page failed");
            return problem(StatusCode::INTERNAL_SERVER_ERROR, "storage failure");
        }
    };

    let has_more = window.len() > limit;
    let window = &window[..window.len().min(limit)];
    let next_since = window.last().map(|s| s.seq).unwrap_or(params.since_seq);

    let envelopes = window
        .iter()
        .filter(|row| row.device_id != params.exclude_device)
        .filter_map(|row| {
            let raw = RawValue::from_string(row.payload.clone()).ok()?;
            Some(SequencedEnvelope {
                seq: row.seq,
                envelope: raw,
            })
        })
        .collect();

    Json(FetchResponse {
        envelopes,
        next_since,
        has_more,
    })
    .into_response()
}

/// RFC 3339 UTC without pulling chrono into the dependency tree.
fn chrono_free_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // civil-from-days (Howard Hinnant's algorithm)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}
