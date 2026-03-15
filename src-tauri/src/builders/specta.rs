use tauri::Wry;
use tauri_specta::{Builder as SpectaBuilder, collect_commands};

pub(crate) fn register_types(builder: SpectaBuilder<Wry>) -> SpectaBuilder<Wry> {
    builder
}

pub(crate) fn register_commands(builder: SpectaBuilder<Wry>) -> SpectaBuilder<Wry> {
    builder
}

#[cfg(all(debug_assertions, desktop))]
pub(crate) fn export(builder: &SpectaBuilder<Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/app/bindings.g.ts",
        )
        .expect("failed to export specta bindings");
}
