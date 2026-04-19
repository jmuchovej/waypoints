use std::sync::Arc;

use domain::chart::ChartID;
use domain::mark::{Content, Mark, RichText};
use tauri::State;
use uuid::Uuid;

use cartograph::WaypointsRuntime;
use cartograph::mark::MarkResult;

#[tauri::command]
pub async fn mark_list(
    rt: State<'_, Arc<WaypointsRuntime>>,
    prefix: Option<String>,
    allowed_types: Option<Vec<ChartID>>,
) -> MarkResult<Vec<Mark>> {
    rt.marks()
        .list(prefix.as_deref(), allowed_types.as_deref())
        .await
}

#[tauri::command]
pub async fn mark_create(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: Uuid,
    parent_id: Uuid,
    content: Content,
    created_at: String,
    after_id: Option<Uuid>,
) -> MarkResult<()> {
    rt.marks()
        .create(id, parent_id, content, created_at, after_id)
        .await
}

#[tauri::command]
pub async fn mark_update(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: Uuid,
    content: Content,
) -> MarkResult<()> {
    rt.marks().update(id, content).await
}

#[tauri::command]
pub async fn mark_delete(rt: State<'_, Arc<WaypointsRuntime>>, id: Uuid) -> MarkResult<()> {
    rt.marks().delete(id).await
}

#[tauri::command]
pub async fn mark_move(
    rt: State<'_, Arc<WaypointsRuntime>>,
    parent_id: Uuid,
    child_id: Uuid,
    after_id: Option<Uuid>,
) -> MarkResult<()> {
    rt.marks().move_mark(parent_id, child_id, after_id).await
}

#[tauri::command]
pub async fn mark_update_title(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: Uuid,
    title: RichText,
) -> MarkResult<()> {
    rt.marks().update_title(id, title).await
}
