//! Territory command handlers. Mirrors
//! `src-tauri/src/commands/territory.rs`.

use super::{ApiError, Instance};
use axum::Json;
use cartograph::Territory;
use serde::Deserialize;
use uuid::Uuid;

pub async fn list(Instance(rt): Instance) -> Result<Json<Vec<Territory>>, ApiError> {
    Ok(Json(rt.territory().list().await?))
}

pub async fn get_active(Instance(rt): Instance) -> Result<Json<Option<Territory>>, ApiError> {
    Ok(Json(rt.territory().get_active().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryCreate {
    name: String,
}

pub async fn create(
    Instance(rt): Instance,
    Json(args): Json<TerritoryCreate>,
) -> Result<Json<Territory>, ApiError> {
    Ok(Json(rt.territory().create(args.name).await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryOpen {
    id: Option<Uuid>,
    path: Option<String>,
    name: Option<String>,
}

pub async fn open(
    Instance(rt): Instance,
    Json(args): Json<TerritoryOpen>,
) -> Result<Json<Territory>, ApiError> {
    Ok(Json(
        rt.territory().open(args.id, args.path, args.name).await?,
    ))
}

pub async fn close(Instance(rt): Instance) -> Result<Json<()>, ApiError> {
    Ok(Json(rt.territory().close().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryRemove {
    id: Uuid,
    delete_data: bool,
}

pub async fn remove(
    Instance(rt): Instance,
    Json(args): Json<TerritoryRemove>,
) -> Result<Json<()>, ApiError> {
    Ok(Json(
        rt.territory().remove(args.id, args.delete_data).await?,
    ))
}
