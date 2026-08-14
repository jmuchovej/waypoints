//! Sync command handlers. Mirrors `waypoints/src-tauri/src/commands/sync.rs`.

use super::{ApiError, Instance};
use axum::Json;
use cartograph::{IdentityCreated, PairingCode, SetupCode, StationStatus, SyncDevice, SyncStatus};
use serde::Deserialize;

pub async fn status(Instance(rt): Instance) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().status().await?))
}

pub async fn create_identity(Instance(rt): Instance) -> Result<Json<IdentityCreated>, ApiError> {
    Ok(Json(rt.sync().identity_create().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSetupCode {
    input: String,
}

pub async fn submit_setup_code(
    Instance(rt): Instance,
    Json(args): Json<SubmitSetupCode>,
) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().submit_setup_code(args.input).await?))
}

pub async fn classify_setup_code(
    Json(args): Json<SubmitSetupCode>,
) -> Result<Json<SetupCode>, ApiError> {
    Ok(Json(cartograph::classify(&args.input)))
}

pub async fn get_identity_mnemomic(Instance(rt): Instance) -> Result<Json<String>, ApiError> {
    Ok(Json(rt.sync().identity_mnemonic().await?))
}

pub async fn create_territory(Instance(rt): Instance) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().territory_create().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationSet {
    addr_json: String,
}

pub async fn set_station(
    Instance(rt): Instance,
    Json(args): Json<StationSet>,
) -> Result<Json<StationStatus>, ApiError> {
    Ok(Json(rt.sync().station_set(args.addr_json).await?))
}

pub async fn station_status(Instance(rt): Instance) -> Result<Json<StationStatus>, ApiError> {
    Ok(Json(rt.sync().station_status().await?))
}

pub async fn now(Instance(rt): Instance) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().sync_now().await?))
}

pub async fn devices(Instance(rt): Instance) -> Result<Json<Vec<SyncDevice>>, ApiError> {
    Ok(Json(rt.sync().devices().await?))
}

pub async fn pairing_code(Instance(rt): Instance) -> Result<Json<PairingCode>, ApiError> {
    Ok(Json(rt.sync().pairing_code().await?))
}

pub async fn recovery_qr(Instance(rt): Instance) -> Result<Json<String>, ApiError> {
    Ok(Json(rt.sync().recovery_qr().await?))
}
