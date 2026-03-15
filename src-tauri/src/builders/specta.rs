use tauri::Wry;
use tauri_specta::{Builder as SpectaBuilder, collect_commands};

use crate::territory::{Territory, TerritoryKind};

pub(crate) fn register_types(builder: SpectaBuilder<Wry>) -> SpectaBuilder<Wry> {
    let builder = builder
        // territory
        .typ::<Territory>()
        .typ::<TerritoryKind>();

    builder
}

pub(crate) fn register_commands(builder: SpectaBuilder<Wry>) -> SpectaBuilder<Wry> {
    use crate::commands::territory;

    let builder = builder.commands(collect_commands![
        territory::territory_get_active,
        territory::territory_list,
        territory::territory_open,
        territory::territory_close,
        territory::territory_create,
        territory::territory_remove,
        // editor
        editor::editor_get_mark,
        editor::editor_mark_list,
        editor::editor_project_landmark,
        editor::editor_create_mark,
        editor::editor_update_mark,
        editor::editor_delete_mark,
        editor::editor_move_mark,
        editor::editor_update_mark_title,
        editor::editor_create_mark_after,
        editor::editor_update_mark_text,
        editor::editor_split_mark,
        editor::editor_merge_marks,
        editor::editor_delete_marks,
        editor::editor_reparent_mark,
        editor::editor_swap_mark_order,
        editor::editor_undo,
        editor::editor_redo,
    ]);

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
