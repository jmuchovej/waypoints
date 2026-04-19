use std::sync::Arc;

use domain::chart::{Chart, ChartID};
use domain::mark::Mark;
use tauri::State;

use cartograph::WaypointsRuntime;
use cartograph::mark::MarkResult;

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
) -> MarkResult<Mark> {
    rt.charts().create_landmark(chart_id).await
}
