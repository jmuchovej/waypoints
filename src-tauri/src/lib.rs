use tauri::{App, Builder as TauriBuilder, Wry};
use tauri_specta::Builder as SpectaBuilder;

mod builders {
    pub(crate) mod specta;
    #[cfg(desktop)]
    pub(crate) mod tray;
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(move |app: &mut App| {
            specta.mount_events(app);

            builders::store::setup(app)?;

            #[cfg(desktop)]
            {
                builders::tray::setup(app)?;

            Ok(())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
