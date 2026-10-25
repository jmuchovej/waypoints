//! HTTP command handlers, organized to mirror `src-tauri/src/commands/`.
//!
//! Each handler is an explicit, typed axum route that hands off to
//! `WaypointsRuntime` immediately — the HTTP analog of the Tauri
//! `#[tauri::command]` of the same name. Routes are registered at
//! `/ipc/<command_name>` so the frontend transport (`app/ipc/`) is identical to
//! the Tauri one. Keep this list in sync with the Tauri `generate_handler!`.

use super::{ApiError, AppState, Instance};
use axum::Router;
use axum::routing::post;

mod chart;
mod editor;
mod mark;
mod sync;
mod territory;

/// Every IPC command route. State is applied by the caller.
pub fn router() -> Router<AppState> {
    Router::new()
        // chart
        .route("/ipc/chart_list", post(chart::list))
        .route("/ipc/chart_get", post(chart::get))
        .route("/ipc/landmark_create", post(chart::landmark_create))
        // mark
        .route("/ipc/mark_list", post(mark::list))
        .route("/ipc/mark_create", post(mark::create))
        .route("/ipc/mark_update", post(mark::update))
        .route("/ipc/mark_delete", post(mark::delete))
        .route("/ipc/mark_move", post(mark::r#move))
        .route("/ipc/mark_update_title", post(mark::update_title))
        // territory
        .route("/ipc/territory_get_active", post(territory::get_active))
        .route("/ipc/territory_list", post(territory::list))
        .route("/ipc/territory_open", post(territory::open))
        .route("/ipc/territory_close", post(territory::close))
        .route("/ipc/territory_create", post(territory::create))
        .route("/ipc/territory_remove", post(territory::remove))
        // sync
        .route("/ipc/sync_status", post(sync::status))
        .route("/ipc/sync_identity_create", post(sync::create_identity))
        .route("/ipc/sync_submit_setup_code", post(sync::submit_setup_code))
        .route(
            "/ipc/sync_classify_setup_code",
            post(sync::classify_setup_code),
        )
        .route(
            "/ipc/sync_identity_mnemonic",
            post(sync::get_identity_mnemomic),
        )
        .route("/ipc/sync_territory_create", post(sync::create_territory))
        .route("/ipc/sync_station_set", post(sync::set_station))
        .route("/ipc/sync_station_status", post(sync::station_status))
        .route("/ipc/sync_now", post(sync::now))
        .route("/ipc/sync_devices", post(sync::devices))
        .route("/ipc/sync_pairing_code", post(sync::pairing_code))
        .route("/ipc/sync_recovery_qr", post(sync::recovery_qr))
        // editor
        .route("/ipc/editor_get_mark", post(editor::get_mark))
        .route("/ipc/editor_mark_list", post(editor::list_marks))
        .route(
            "/ipc/editor_project_landmark",
            post(editor::project_landmark),
        )
        .route("/ipc/editor_update_mark", post(editor::update_mark))
        .route("/ipc/editor_delete_mark", post(editor::delete_mark))
        .route("/ipc/editor_move_marks", post(editor::move_marks))
        .route(
            "/ipc/editor_update_mark_title",
            post(editor::update_mark_title),
        )
        .route(
            "/ipc/editor_update_mark_reading",
            post(editor::update_mark_reading),
        )
        .route(
            "/ipc/editor_wrap_in_compound",
            post(editor::wrap_in_compound),
        )
        .route(
            "/ipc/editor_create_mark_after",
            post(editor::create_mark_after),
        )
        .route(
            "/ipc/editor_update_mark_text",
            post(editor::update_mark_text),
        )
        .route("/ipc/editor_split_mark", post(editor::split_mark))
        .route("/ipc/editor_merge_marks", post(editor::merge_marks))
        .route("/ipc/editor_delete_marks", post(editor::delete_marks))
        .route("/ipc/editor_reparent_mark", post(editor::reparent_mark))
        .route("/ipc/editor_undo", post(editor::undo))
        .route("/ipc/editor_redo", post(editor::redo))
}
