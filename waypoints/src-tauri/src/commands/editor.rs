use std::sync::Arc;

use domain::chart::FeatureID;
use domain::mark::mark_data::MathEngine;
use domain::mark::{Content, Mark, MarkData, MarkID, Reading, RichText};
use tauri::State;

use cartograph::WaypointsRuntime;
use editor::{CreateResult, EditorResult, MutationResult, RenderNode, SplitResult, UndoRedoResult};

#[tauri::command]
pub async fn editor_get_mark(
    rt: State<'_, Arc<WaypointsRuntime>>,
    mark_id: MarkID,
) -> EditorResult<Mark> {
    rt.editor().await.get_mark(mark_id).await
}

#[tauri::command]
pub async fn editor_mark_list(rt: State<'_, Arc<WaypointsRuntime>>) -> EditorResult<Vec<Mark>> {
    rt.editor().await.mark_list().await
}

#[tauri::command]
pub async fn editor_project_landmark(
    rt: State<'_, Arc<WaypointsRuntime>>,
    landmark_id: MarkID,
    max_depth: Option<u32>,
) -> EditorResult<Vec<RenderNode>> {
    rt.editor()
        .await
        .project_landmark(landmark_id, max_depth)
        .await
}

#[tauri::command]
pub async fn editor_update_mark(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: MarkID,
    content: Content,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    rt.editor()
        .await
        .update_mark(id, content, landmark_id)
        .await
}

#[tauri::command]
pub async fn editor_delete_mark(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    rt.editor().await.delete_mark(id, landmark_id).await
}

#[tauri::command]
pub async fn editor_move_marks(
    rt: State<'_, Arc<WaypointsRuntime>>,
    parent_id: MarkID,
    child_ids: Vec<MarkID>,
    after_id: Option<MarkID>,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    rt.editor()
        .await
        .move_marks(parent_id, &child_ids, after_id, landmark_id)
        .await
}

#[tauri::command]
pub async fn editor_update_mark_title(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: MarkID,
    title: RichText,
) -> EditorResult<()> {
    rt.editor().await.update_mark_title(id, title).await
}

#[tauri::command]
pub async fn editor_update_mark_reading(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: MarkID,
    feature_id: FeatureID,
    reading: Reading,
) -> EditorResult<Mark> {
    rt.editor()
        .await
        .update_mark_reading(id, feature_id, reading)
        .await
}

#[tauri::command]
pub async fn editor_create_mark_after(
    rt: State<'_, Arc<WaypointsRuntime>>,
    after_id: MarkID,
    parent_id: MarkID,
    data: MarkData,
    landmark_id: MarkID,
) -> EditorResult<CreateResult> {
    rt.editor()
        .await
        .create_mark_after(after_id, parent_id, data, landmark_id)
        .await
}

#[tauri::command]
pub async fn editor_update_mark_text(
    rt: State<'_, Arc<WaypointsRuntime>>,
    mark_id: MarkID,
    rich_text: RichText,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    rt.editor()
        .await
        .update_mark_text(mark_id, rich_text, landmark_id)
        .await
}

#[tauri::command]
pub async fn editor_split_mark(
    rt: State<'_, Arc<WaypointsRuntime>>,
    mark_id: MarkID,
    offset: u32,
    landmark_id: MarkID,
) -> EditorResult<SplitResult> {
    rt.editor()
        .await
        .split_mark(mark_id, offset, landmark_id)
        .await
}

#[tauri::command]
pub async fn editor_merge_marks(
    rt: State<'_, Arc<WaypointsRuntime>>,
    target_id: MarkID,
    source_id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    rt.editor()
        .await
        .merge_marks(target_id, source_id, landmark_id)
        .await
}

#[tauri::command]
pub async fn editor_delete_marks(
    rt: State<'_, Arc<WaypointsRuntime>>,
    mark_ids: Vec<MarkID>,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    rt.editor().await.delete_marks(&mark_ids, landmark_id).await
}

#[tauri::command]
pub async fn editor_reparent_mark(
    rt: State<'_, Arc<WaypointsRuntime>>,
    mark_id: MarkID,
    new_parent_id: MarkID,
    after_id: Option<MarkID>,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    rt.editor()
        .await
        .reparent_mark(mark_id, new_parent_id, after_id, landmark_id)
        .await
}

#[tauri::command]
pub async fn editor_undo(
    rt: State<'_, Arc<WaypointsRuntime>>,
    landmark_id: MarkID,
) -> EditorResult<Option<UndoRedoResult>> {
    rt.editor().await.undo(landmark_id).await
}

#[tauri::command]
pub async fn editor_redo(
    rt: State<'_, Arc<WaypointsRuntime>>,
    landmark_id: MarkID,
) -> EditorResult<Option<UndoRedoResult>> {
    rt.editor().await.redo(landmark_id).await
}

#[tauri::command]
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
