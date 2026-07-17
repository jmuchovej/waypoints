use std::sync::Arc;

use tauri::State;

use cartograph::WaypointsRuntime;
use cartograph::sync::{IdentityCreated, StationStatus, SyncDevice, SyncResult, SyncStatus};

#[tauri::command]
pub async fn sync_status(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<SyncStatus> {
    rt.sync().status().await
}

#[tauri::command]
pub async fn sync_identity_create(
    rt: State<'_, Arc<WaypointsRuntime>>,
) -> SyncResult<IdentityCreated> {
    rt.sync().identity_create().await
}

#[tauri::command]
pub async fn sync_identity_import(
    rt: State<'_, Arc<WaypointsRuntime>>,
    phrase: String,
) -> SyncResult<SyncStatus> {
    rt.sync().identity_import(phrase).await
}

#[tauri::command]
pub async fn sync_identity_mnemonic(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<String> {
    rt.sync().identity_mnemonic().await
}

#[tauri::command]
pub async fn sync_territory_create(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<SyncStatus> {
    rt.sync().territory_create().await
}

#[tauri::command]
pub async fn sync_station_set(
    rt: State<'_, Arc<WaypointsRuntime>>,
    addr_json: String,
) -> SyncResult<StationStatus> {
    rt.sync().station_set(addr_json).await
}

#[tauri::command]
pub async fn sync_station_status(
    rt: State<'_, Arc<WaypointsRuntime>>,
) -> SyncResult<StationStatus> {
    rt.sync().station_status().await
}

#[tauri::command]
pub async fn sync_now(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<SyncStatus> {
    rt.sync().sync_now().await
}

#[tauri::command]
pub async fn sync_devices(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<Vec<SyncDevice>> {
    rt.sync().devices().await
}

#[tauri::command]
pub async fn sync_pairing_payload(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<String> {
    rt.sync().pairing_payload().await
}

#[tauri::command]
pub async fn sync_pairing_join(
    rt: State<'_, Arc<WaypointsRuntime>>,
    code: String,
) -> SyncResult<SyncStatus> {
    rt.sync().pairing_join(code).await
}
