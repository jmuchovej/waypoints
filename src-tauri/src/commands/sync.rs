use cartograph::{
    IdentityCreated, PairingCode, SetupCode, StationStatus, SyncDevice, SyncResult, SyncState,
    SyncStatus, WaypointsRuntime,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn sync_status(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<SyncStatus> {
    rt.sync().status().await
}

#[tauri::command]
pub async fn sync_state(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<SyncState> {
    rt.sync().sync_state().await
}

#[tauri::command]
pub async fn sync_identity_create(
    rt: State<'_, Arc<WaypointsRuntime>>,
) -> SyncResult<IdentityCreated> {
    rt.sync().identity_create().await
}

#[tauri::command]
pub async fn sync_submit_setup_code(
    rt: State<'_, Arc<WaypointsRuntime>>,
    input: String,
) -> SyncResult<SyncStatus> {
    rt.sync().submit_setup_code(input).await
}

#[tauri::command]
pub async fn sync_classify_setup_code(input: String) -> SyncResult<SetupCode> {
    Ok(cartograph::classify(&input))
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
pub async fn sync_pairing_code(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<PairingCode> {
    rt.sync().pairing_code().await
}

#[tauri::command]
pub async fn sync_recovery_qr(rt: State<'_, Arc<WaypointsRuntime>>) -> SyncResult<String> {
    rt.sync().recovery_qr().await
}
