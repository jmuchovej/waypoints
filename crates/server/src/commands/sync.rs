//! Sync command handlers. Mirrors `waypoints/src-tauri/src/commands/sync.rs`.

use axum::Json;
use serde::Deserialize;

use cartograph::sync::{IdentityCreated, StationStatus, SyncDevice, SyncStatus};

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
pub struct IdentityImport {
    phrase: String,
}

pub async fn sync_identity_import(
    Instance(rt): Instance,
    Json(args): Json<IdentityImport>,
) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().identity_import(args.phrase).await?))
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

pub async fn sync_pairing_payload(Instance(rt): Instance) -> Result<Json<String>, ApiError> {
    Ok(Json(rt.sync().pairing_payload().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingJoin {
    code: String,
}

pub async fn sync_pairing_join(
    Instance(rt): Instance,
    Json(args): Json<PairingJoin>,
) -> Result<Json<SyncStatus>, ApiError> {
    Ok(Json(rt.sync().pairing_join(args.code).await?))
}
