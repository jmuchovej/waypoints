use tauri::{App, Builder as TauriBuilder, Wry};
use tauri_specta::Builder as SpectaBuilder;

mod builders {
    pub(crate) mod specta;
    #[cfg(desktop)]
    pub(crate) mod tray;
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("waypoints starting");

    let specta = {
        let builder = SpectaBuilder::<Wry>::new();
        let builder = builders::specta::register_types(builder);
        let builder = builders::specta::register_commands(builder);

        #[cfg(all(debug_assertions, desktop))]
        builders::specta::export(&builder);

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

            tracing::info!("tauri setup: initializing store");
            builders::store::setup(app)?;
            tracing::info!("tauri setup: store ready");

            #[cfg(desktop)]
            {
                tracing::info!("tauri setup: building tray icon");
                builders::tray::setup(app)?;
                tracing::info!("tauri setup: tray ready");
            }

            tracing::info!("tauri setup: complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
