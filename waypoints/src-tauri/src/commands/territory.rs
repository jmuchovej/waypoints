use cartograph::{Territory, TerritoryResult, WaypointsRuntime};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn territory_list(
    rt: State<'_, Arc<WaypointsRuntime>>,
) -> TerritoryResult<Vec<Territory>> {
    rt.territory().list().await
}

#[tauri::command]
pub async fn territory_get_active(
    rt: State<'_, Arc<WaypointsRuntime>>,
) -> TerritoryResult<Option<Territory>> {
    rt.territory().get_active().await
}

#[tauri::command]
pub async fn territory_create(
    rt: State<'_, Arc<WaypointsRuntime>>,
    name: String,
) -> TerritoryResult<Territory> {
    rt.territory().create(name).await
}

#[tauri::command]
pub async fn territory_open(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: Option<Uuid>,
    path: Option<String>,
    name: Option<String>,
) -> TerritoryResult<Territory> {
    rt.territory().open(id, path, name).await
}

#[tauri::command]
pub async fn territory_close(rt: State<'_, Arc<WaypointsRuntime>>) -> TerritoryResult<()> {
    rt.territory().close().await
}

#[tauri::command]
pub async fn territory_remove(
    rt: State<'_, Arc<WaypointsRuntime>>,
    id: Uuid,
    delete_data: bool,
) -> TerritoryResult<()> {
    rt.territory().remove(id, delete_data).await
}
