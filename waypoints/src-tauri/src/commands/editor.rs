use domain::chart::FeatureID;
use domain::mark::mark_data::MathEngine;
use domain::mark::{Content, Mark, MarkData, MarkID, Reading, RichText};
use tauri::State;
use tokio::sync::Mutex;

use editor::{
    CreateResult, EditorResult, EditorService, MutationResult, RenderNode, SplitResult,
    UndoRedoResult,
};

#[tauri::command]
#[specta::specta]
pub async fn editor_get_mark(
    editor: State<'_, Mutex<EditorService>>,
    mark_id: MarkID,
) -> EditorResult<Mark> {
    editor.lock().await.get_mark(mark_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_mark_list(editor: State<'_, Mutex<EditorService>>) -> EditorResult<Vec<Mark>> {
    editor.lock().await.mark_list().await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_project_landmark(
    editor: State<'_, Mutex<EditorService>>,
    landmark_id: MarkID,
    max_depth: Option<u32>,
) -> EditorResult<Vec<RenderNode>> {
    editor
        .lock()
        .await
        .project_landmark(landmark_id, max_depth)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_update_mark(
    editor: State<'_, Mutex<EditorService>>,
    id: MarkID,
    content: Content,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    editor
        .lock()
        .await
        .update_mark(id, content, landmark_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_delete_mark(
    editor: State<'_, Mutex<EditorService>>,
    id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    editor.lock().await.delete_mark(id, landmark_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_move_marks(
    editor: State<'_, Mutex<EditorService>>,
    parent_id: MarkID,
    child_ids: Vec<MarkID>,
    after_id: Option<MarkID>,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    editor
        .lock()
        .await
        .move_marks(parent_id, &child_ids, after_id, landmark_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_update_mark_title(
    editor: State<'_, Mutex<EditorService>>,
    id: MarkID,
    title: RichText,
) -> EditorResult<()> {
    editor.lock().await.update_mark_title(id, title).await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_update_mark_reading(
    editor: State<'_, Mutex<EditorService>>,
    id: MarkID,
    feature_id: FeatureID,
    reading: Reading,
) -> EditorResult<Mark> {
    editor
        .lock()
        .await
        .update_mark_reading(id, feature_id, reading)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_create_mark_after(
    editor: State<'_, Mutex<EditorService>>,
    after_id: MarkID,
    parent_id: MarkID,
    data: MarkData,
    landmark_id: MarkID,
) -> EditorResult<CreateResult> {
    editor
        .lock()
        .await
        .create_mark_after(after_id, parent_id, data, landmark_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_update_mark_text(
    editor: State<'_, Mutex<EditorService>>,
    mark_id: MarkID,
    rich_text: RichText,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    editor
        .lock()
        .await
        .update_mark_text(mark_id, rich_text, landmark_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_split_mark(
    editor: State<'_, Mutex<EditorService>>,
    mark_id: MarkID,
    offset: u32,
    landmark_id: MarkID,
) -> EditorResult<SplitResult> {
    editor
        .lock()
        .await
        .split_mark(mark_id, offset, landmark_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_merge_marks(
    editor: State<'_, Mutex<EditorService>>,
    target_id: MarkID,
    source_id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    editor
        .lock()
        .await
        .merge_marks(target_id, source_id, landmark_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_delete_marks(
    editor: State<'_, Mutex<EditorService>>,
    mark_ids: Vec<MarkID>,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    editor
        .lock()
        .await
        .delete_marks(&mark_ids, landmark_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_reparent_mark(
    editor: State<'_, Mutex<EditorService>>,
    mark_id: MarkID,
    new_parent_id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    editor
        .lock()
        .await
        .reparent_mark(mark_id, new_parent_id, landmark_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_undo(
    editor: State<'_, Mutex<EditorService>>,
    landmark_id: MarkID,
) -> EditorResult<Option<UndoRedoResult>> {
    editor.lock().await.undo(landmark_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_redo(
    editor: State<'_, Mutex<EditorService>>,
    landmark_id: MarkID,
) -> EditorResult<Option<UndoRedoResult>> {
    editor.lock().await.redo(landmark_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn editor_render_math(
    expression: String,
    engine: MathEngine,
    display_mode: bool,
) -> Result<String, String> {
    match engine {
        MathEngine::Typst => tokio::task::spawn_blocking(move || {
            editor::typst::render_math_to_svg(&expression, display_mode)
        })
        .await
        .map_err(|e| format!("task join error: {e}"))?,
        MathEngine::LaTeX => Err("LaTeX rendering is client-side via KaTeX".into()),
    }
}
