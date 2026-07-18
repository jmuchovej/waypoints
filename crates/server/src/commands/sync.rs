//! Sync command handlers. Mirrors `waypoints/src-tauri/src/commands/sync.rs`.

use axum::Json;
use serde::Deserialize;

use cartograph::sync::{
    IdentityCreated, PairingCode, SetupCode, StationStatus, SyncDevice, SyncStatus,
};

use crate::error::ApiError;
use crate::instance::Instance;

pub async fn sync_status(Instance(rt): Instance) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().status().await?))
}

pub async fn sync_identity_create(
    Instance(rt): Instance,
) -> Result<Json<IdentityCreated>, ApiError> {
    Ok(Json(rt.sync().identity_create().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSetupCode {
    input: String,
}

pub async fn sync_submit_setup_code(
    Instance(rt): Instance,
    Json(args): Json<SubmitSetupCode>,
) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().submit_setup_code(args.input).await?))
}

pub async fn sync_classify_setup_code(
    Json(args): Json<SubmitSetupCode>,
) -> Result<Json<SetupCode>, ApiError> {
    Ok(Json(cartograph::sync::classify(&args.input)))
}

pub async fn sync_identity_mnemonic(Instance(rt): Instance) -> Result<Json<String>, ApiError> {
    Ok(Json(rt.sync().identity_mnemonic().await?))
}

pub async fn sync_territory_create(Instance(rt): Instance) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().territory_create().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationSet {
    addr_json: String,
}

pub async fn sync_station_set(
    Instance(rt): Instance,
    Json(args): Json<StationSet>,
) -> Result<Json<StationStatus>, ApiError> {
    Ok(Json(rt.sync().station_set(args.addr_json).await?))
}

pub async fn sync_station_status(Instance(rt): Instance) -> Result<Json<StationStatus>, ApiError> {
    Ok(Json(rt.sync().station_status().await?))
}

pub async fn sync_now(Instance(rt): Instance) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().sync_now().await?))
}

pub async fn sync_devices(Instance(rt): Instance) -> Result<Json<Vec<SyncDevice>>, ApiError> {
    Ok(Json(rt.sync().devices().await?))
}

pub async fn sync_pairing_code(Instance(rt): Instance) -> Result<Json<PairingCode>, ApiError> {
    Ok(Json(rt.sync().pairing_code().await?))
}

pub async fn sync_recovery_qr(Instance(rt): Instance) -> Result<Json<String>, ApiError> {
    Ok(Json(rt.sync().recovery_qr().await?))
}
