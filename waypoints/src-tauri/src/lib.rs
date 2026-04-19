use tauri::{App, Builder as TauriBuilder};

mod registry_store;
mod settings;

pub mod builders {
    #[cfg(desktop)]
    pub(crate) mod menus;
    pub(crate) mod territory;
    #[cfg(desktop)]
    pub(crate) mod tray;
}

mod commands {
    pub mod chart;
    pub mod editor;
    pub mod mark;
    pub mod territory;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("waypoints starting");

    TauriBuilder::default()
        .invoke_handler(tauri::generate_handler![
            // chart
            commands::chart::chart_list,
            commands::chart::chart_get,
            commands::chart::landmark_create,
            // mark
            commands::mark::mark_list,
            commands::mark::mark_create,
            commands::mark::mark_update,
            commands::mark::mark_delete,
            commands::mark::mark_move,
            commands::mark::mark_update_title,
            // territory
            commands::territory::territory_get_active,
            commands::territory::territory_list,
            commands::territory::territory_open,
            commands::territory::territory_close,
            commands::territory::territory_create,
            commands::territory::territory_remove,
            // editor
            commands::editor::editor_get_mark,
            commands::editor::editor_mark_list,
            commands::editor::editor_project_landmark,
            commands::editor::editor_update_mark,
            commands::editor::editor_delete_mark,
            commands::editor::editor_move_marks,
            commands::editor::editor_update_mark_title,
            commands::editor::editor_update_mark_reading,
            commands::editor::editor_wrap_in_compound,
            commands::editor::editor_create_mark_after,
            commands::editor::editor_update_mark_text,
            commands::editor::editor_split_mark,
            commands::editor::editor_merge_marks,
            commands::editor::editor_delete_marks,
            commands::editor::editor_reparent_mark,
            commands::editor::editor_undo,
            commands::editor::editor_redo,
            commands::editor::editor_render_math,
        ])
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(move |app: &mut App| {
            tracing::info!("tauri setup: initializing runtime");
            builders::territory::setup(app)?;
            tracing::info!("tauri setup: runtime ready");

            #[cfg(desktop)]
            {
                tracing::info!("tauri setup: building tray icon");
                builders::tray::setup(app)?;
                tracing::info!("tauri setup: tray ready");

                tracing::info!("tauri setup: building dev-only menus");
                builders::menus::setup(app)?;
                tracing::info!("tauri setup: dev-only menus ready");
            }

            tracing::info!("tauri setup: complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
