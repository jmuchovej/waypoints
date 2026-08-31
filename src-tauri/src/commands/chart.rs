use cartograph::{EditorResult, MarkResult, WaypointsRuntime};
use domain::{Chart, ChartID, Mark};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn chart_list(rt: State<'_, Arc<WaypointsRuntime>>) -> MarkResult<Vec<Chart>> {
    rt.charts().list().await
}

#[tauri::command]
pub async fn chart_get(
    rt: State<'_, Arc<WaypointsRuntime>>,
    chart_id: ChartID,
) -> MarkResult<Option<Chart>> {
    rt.charts().get(chart_id).await
}

#[tauri::command]
pub async fn landmark_create(
    rt: State<'_, Arc<WaypointsRuntime>>,
    chart_id: ChartID,
) -> EditorResult<Mark> {
    rt.editor().create_landmark(chart_id).await
}
