//! Chart command handlers. Mirrors `waypoints/src-tauri/src/commands/chart.rs`.

use axum::Json;
use serde::Deserialize;

use domain::chart::{Chart, ChartID};
use domain::mark::Mark;

use crate::error::ApiError;
use crate::instance::Instance;

pub async fn chart_list(Instance(rt): Instance) -> Result<Json<Vec<Chart>>, ApiError> {
    Ok(Json(rt.charts().list().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartId {
    chart_id: ChartID,
}

pub async fn chart_get(
    Instance(rt): Instance,
    Json(args): Json<ChartId>,
) -> Result<Json<Option<Chart>>, ApiError> {
    Ok(Json(rt.charts().get(args.chart_id).await?))
}

pub async fn landmark_create(
    Instance(rt): Instance,
    Json(args): Json<ChartId>,
) -> Result<Json<Mark>, ApiError> {
    Ok(Json(rt.charts().create_landmark(args.chart_id).await?))
}
