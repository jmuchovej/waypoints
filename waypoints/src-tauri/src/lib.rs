use tauri::{App, Builder as TauriBuilder};

mod registry_store;
mod settings;

pub mod builders {
    #[cfg(desktop)]
    pub(crate) mod menus;
    pub mod specta;
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

    let specta = {
        let builder = builders::specta::build();

        #[cfg(all(debug_assertions, desktop))]
        builders::specta::export(&builder, "../app/bindings.g.ts");

        builder
    };

    TauriBuilder::default()
        .invoke_handler(specta.invoke_handler())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(move |app: &mut App| {
            tracing::info!("tauri setup: mounting specta events");
            specta.mount_events(app);

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
