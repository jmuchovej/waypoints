use tauri::Wry;
use tauri_specta::{Builder as SpectaBuilder, collect_commands};

use crate::mark::MarkTree;
use crate::territory::{Territory, TerritoryKind};

pub fn register_types(builder: SpectaBuilder<Wry>) -> SpectaBuilder<Wry> {
    let builder = builder
        // mark (expanded)
        .typ::<MarkTree>()
        // territory
        .typ::<Territory>()
        .typ::<TerritoryKind>();

    builder
}

pub fn register_commands(builder: SpectaBuilder<Wry>) -> SpectaBuilder<Wry> {
    use crate::commands::{editor, mark, territory};

    let builder = builder.commands(collect_commands![
        mark::mark_list,
        mark::mark_get,
        mark::mark_create,
        mark::mark_update,
        mark::mark_delete,
        mark::mark_move,
        mark::mark_update_title,
        // territory
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

pub fn export(builder: &SpectaBuilder<Wry>, path: &str) {
    builder
        .export(specta_typescript::Typescript::default(), path)
        .expect("failed to export specta bindings");
}

pub fn build() -> SpectaBuilder<Wry> {
    let builder = SpectaBuilder::<Wry>::new();
    let builder = register_types(builder);
    register_commands(builder)
}
