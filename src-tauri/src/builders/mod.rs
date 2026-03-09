use tauri::App;

#[cfg(desktop)]
mod tray;

pub(crate) fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(desktop)]
    tray::setup(app)?;

    Ok(())
}
