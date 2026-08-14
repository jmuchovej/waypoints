//! Mark command handlers. Mirrors `src-tauri/src/commands/mark.rs`.

use super::{ApiError, Instance};
use axum::Json;
use domain::{ChartID, Content, Mark, RichText};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkList {
    prefix: Option<String>,
    allowed_types: Option<Vec<ChartID>>,
}

pub async fn list(
    Instance(rt): Instance,
    Json(args): Json<MarkList>,
) -> Result<Json<Vec<Mark>>, ApiError> {
    Ok(Json(
        rt.marks()
            .list(args.prefix.as_deref(), args.allowed_types.as_deref())
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkCreate {
    id: Uuid,
    parent_id: Uuid,
    content: Content,
    created_at: String,
    after_id: Option<Uuid>,
}

pub async fn create(
    Instance(rt): Instance,
    Json(args): Json<MarkCreate>,
) -> Result<Json<()>, ApiError> {
    Ok(Json(
        rt.marks()
            .create(
                args.id,
                args.parent_id,
                args.content,
                args.created_at,
                args.after_id,
            )
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkUpdate {
    id: Uuid,
    content: Content,
}

pub async fn update(
    Instance(rt): Instance,
    Json(args): Json<MarkUpdate>,
) -> Result<Json<()>, ApiError> {
    Ok(Json(rt.marks().update(args.id, args.content).await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkDelete {
    id: Uuid,
}

pub async fn delete(
    Instance(rt): Instance,
    Json(args): Json<MarkDelete>,
) -> Result<Json<()>, ApiError> {
    Ok(Json(rt.marks().delete(args.id).await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkMove {
    parent_id: Uuid,
    child_id: Uuid,
    after_id: Option<Uuid>,
}

pub async fn r#move(
    Instance(rt): Instance,
    Json(args): Json<MarkMove>,
) -> Result<Json<()>, ApiError> {
    Ok(Json(
        rt.marks()
            .move_mark(args.parent_id, args.child_id, args.after_id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkUpdateTitle {
    id: Uuid,
    title: RichText,
}

pub async fn update_title(
    Instance(rt): Instance,
    Json(args): Json<MarkUpdateTitle>,
) -> Result<Json<()>, ApiError> {
    Ok(Json(rt.marks().update_title(args.id, args.title).await?))
}
