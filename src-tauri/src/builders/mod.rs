use tauri::App;

mod specta;
#[cfg(desktop)]
mod tray;

pub(crate) fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    specta::builder()
}

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(desktop)]
    tray::setup(app)?;

    Ok(())
}
