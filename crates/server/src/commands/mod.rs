//! HTTP command handlers, organized to mirror `waypoints/src-tauri/src/commands/`.
//!
//! Each handler is an explicit, typed axum route that hands off to
//! `WaypointsRuntime` immediately — the HTTP analog of the Tauri
//! `#[tauri::command]` of the same name. Routes are registered at
//! `/ipc/<command_name>` so the frontend transport (`app/ipc/`) is identical to
//! the Tauri one. Keep this list in sync with the Tauri `generate_handler!`.

use axum::Router;
use axum::routing::post;

use crate::instance::AppState;

pub mod chart;
pub mod editor;
pub mod mark;
pub mod sync;
pub mod territory;

/// Every IPC command route. State is applied by the caller.
pub fn router() -> Router<AppState> {
    Router::new()
        // chart
        .route("/ipc/chart_list", post(chart::chart_list))
        .route("/ipc/chart_get", post(chart::chart_get))
        .route("/ipc/landmark_create", post(chart::landmark_create))
        // mark
        .route("/ipc/mark_list", post(mark::mark_list))
        .route("/ipc/mark_create", post(mark::mark_create))
        .route("/ipc/mark_update", post(mark::mark_update))
        .route("/ipc/mark_delete", post(mark::mark_delete))
        .route("/ipc/mark_move", post(mark::mark_move))
        .route("/ipc/mark_update_title", post(mark::mark_update_title))
        // territory
        .route(
            "/ipc/territory_get_active",
            post(territory::territory_get_active),
        )
        .route("/ipc/territory_list", post(territory::territory_list))
        .route("/ipc/territory_open", post(territory::territory_open))
        .route("/ipc/territory_close", post(territory::territory_close))
        .route("/ipc/territory_create", post(territory::territory_create))
        .route("/ipc/territory_remove", post(territory::territory_remove))
        // sync
        .route("/ipc/sync_status", post(sync::sync_status))
        .route(
            "/ipc/sync_identity_create",
            post(sync::sync_identity_create),
        )
        .route(
            "/ipc/sync_identity_import",
            post(sync::sync_identity_import),
        )
        .route(
            "/ipc/sync_identity_mnemonic",
            post(sync::sync_identity_mnemonic),
        )
        .route(
            "/ipc/sync_territory_create",
            post(sync::sync_territory_create),
        )
        .route("/ipc/sync_station_set", post(sync::sync_station_set))
        .route("/ipc/sync_station_status", post(sync::sync_station_status))
        .route("/ipc/sync_now", post(sync::sync_now))
        .route("/ipc/sync_devices", post(sync::sync_devices))
        .route(
            "/ipc/sync_pairing_payload",
            post(sync::sync_pairing_payload),
        )
        .route("/ipc/sync_pairing_join", post(sync::sync_pairing_join))
        // editor
        .route("/ipc/editor_get_mark", post(editor::editor_get_mark))
        .route("/ipc/editor_mark_list", post(editor::editor_mark_list))
        .route(
            "/ipc/editor_project_landmark",
            post(editor::editor_project_landmark),
        )
        .route("/ipc/editor_update_mark", post(editor::editor_update_mark))
        .route("/ipc/editor_delete_mark", post(editor::editor_delete_mark))
        .route("/ipc/editor_move_marks", post(editor::editor_move_marks))
        .route(
            "/ipc/editor_update_mark_title",
            post(editor::editor_update_mark_title),
        )
        .route(
            "/ipc/editor_update_mark_reading",
            post(editor::editor_update_mark_reading),
        )
        .route(
            "/ipc/editor_wrap_in_compound",
            post(editor::editor_wrap_in_compound),
        )
        .route(
            "/ipc/editor_create_mark_after",
            post(editor::editor_create_mark_after),
        )
        .route(
            "/ipc/editor_update_mark_text",
            post(editor::editor_update_mark_text),
        )
        .route("/ipc/editor_split_mark", post(editor::editor_split_mark))
        .route("/ipc/editor_merge_marks", post(editor::editor_merge_marks))
        .route(
            "/ipc/editor_delete_marks",
            post(editor::editor_delete_marks),
        )
        .route(
            "/ipc/editor_reparent_mark",
            post(editor::editor_reparent_mark),
        )
        .route("/ipc/editor_undo", post(editor::editor_undo))
        .route("/ipc/editor_redo", post(editor::editor_redo))
}
