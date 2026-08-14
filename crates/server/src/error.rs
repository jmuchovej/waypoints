//! HTTP-facing error for command handlers.
//!
//! cartograph's domain errors (`EditorError` / `MarkError` / `TerritoryError`)
//! serialize as plain message strings — the same contract the Tauri boundary
//! uses. We surface them as 422 with the message as the body, which the
//! frontend's `IPCError` carries through. Handlers just `?` and these `From`
//! impls do the rest.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use cartograph::{EditorError, MarkError, SyncError, TerritoryError};

pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

impl From<EditorError> for ApiError {
    fn from(e: EditorError) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: e.to_string(),
        }
    }
}

impl From<MarkError> for ApiError {
    fn from(e: MarkError) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: e.to_string(),
        }
    }
}

impl From<SyncError> for ApiError {
    fn from(e: SyncError) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: e.to_string(),
        }
    }
}

impl From<TerritoryError> for ApiError {
    fn from(e: TerritoryError) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: e.to_string(),
        }
    }
}
