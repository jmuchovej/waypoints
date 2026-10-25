//! Editor command handlers. Mirrors
//! `src-tauri/src/commands/editor.rs`.

use super::{ApiError, Instance};
use axum::Json;
use cartograph::{CreateResult, MutationResult, RenderNode, SplitResult, UndoRedoResult};
use domain::{Content, FeatureID, Mark, MarkData, MarkID, Reading, RichText};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMark {
    mark_id: MarkID,
}

pub async fn get_mark(
    Instance(rt): Instance,
    Json(args): Json<GetMark>,
) -> Result<Json<Mark>, ApiError> {
    Ok(Json(rt.editor().get_mark(args.mark_id).await?))
}

pub async fn list_marks(Instance(rt): Instance) -> Result<Json<Vec<Mark>>, ApiError> {
    Ok(Json(rt.editor().mark_list().await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLandmark {
    landmark_id: MarkID,
    max_depth: Option<u32>,
}

pub async fn project_landmark(
    Instance(rt): Instance,
    Json(args): Json<ProjectLandmark>,
) -> Result<Json<Vec<RenderNode>>, ApiError> {
    Ok(Json(
        rt.editor()
            .project_landmark(args.landmark_id, args.max_depth)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMark {
    id: MarkID,
    content: Content,
    landmark_id: MarkID,
}

pub async fn update_mark(
    Instance(rt): Instance,
    Json(args): Json<UpdateMark>,
) -> Result<Json<MutationResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .update_mark(args.id, args.content, args.landmark_id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMark {
    id: MarkID,
    landmark_id: MarkID,
}

pub async fn delete_mark(
    Instance(rt): Instance,
    Json(args): Json<DeleteMark>,
) -> Result<Json<MutationResult>, ApiError> {
    Ok(Json(
        rt.editor().delete_mark(args.id, args.landmark_id).await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveMarks {
    parent_id: MarkID,
    child_ids: Vec<MarkID>,
    after_id: Option<MarkID>,
    landmark_id: MarkID,
}

pub async fn move_marks(
    Instance(rt): Instance,
    Json(args): Json<MoveMarks>,
) -> Result<Json<MutationResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .move_marks(
                args.parent_id,
                &args.child_ids,
                args.after_id,
                args.landmark_id,
            )
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMarkTitle {
    id: MarkID,
    title: RichText,
}

pub async fn update_mark_title(
    Instance(rt): Instance,
    Json(args): Json<UpdateMarkTitle>,
) -> Result<Json<()>, ApiError> {
    Ok(Json(
        rt.editor().update_mark_title(args.id, args.title).await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMarkReading {
    id: MarkID,
    feature_id: FeatureID,
    reading: Reading,
}

pub async fn update_mark_reading(
    Instance(rt): Instance,
    Json(args): Json<UpdateMarkReading>,
) -> Result<Json<Mark>, ApiError> {
    Ok(Json(
        rt.editor()
            .update_mark_reading(args.id, args.feature_id, args.reading)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapInCompound {
    compound_data: MarkData,
    child_ids: Vec<MarkID>,
    after_id: Option<MarkID>,
    parent_id: MarkID,
    landmark_id: MarkID,
}

pub async fn wrap_in_compound(
    Instance(rt): Instance,
    Json(args): Json<WrapInCompound>,
) -> Result<Json<CreateResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .wrap_in_compound(
                args.compound_data,
                &args.child_ids,
                args.after_id,
                args.parent_id,
                args.landmark_id,
            )
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMarkAfter {
    after_id: MarkID,
    parent_id: MarkID,
    data: MarkData,
    landmark_id: MarkID,
}

pub async fn create_mark_after(
    Instance(rt): Instance,
    Json(args): Json<CreateMarkAfter>,
) -> Result<Json<CreateResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .create_mark_after(args.after_id, args.parent_id, args.data, args.landmark_id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMarkText {
    mark_id: MarkID,
    rich_text: RichText,
    landmark_id: MarkID,
}

pub async fn update_mark_text(
    Instance(rt): Instance,
    Json(args): Json<UpdateMarkText>,
) -> Result<Json<MutationResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .update_mark_text(args.mark_id, args.rich_text, args.landmark_id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitMark {
    mark_id: MarkID,
    offset: u32,
    landmark_id: MarkID,
}

pub async fn split_mark(
    Instance(rt): Instance,
    Json(args): Json<SplitMark>,
) -> Result<Json<SplitResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .split_mark(args.mark_id, args.offset, args.landmark_id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeMarks {
    target_id: MarkID,
    source_id: MarkID,
    landmark_id: MarkID,
}

pub async fn merge_marks(
    Instance(rt): Instance,
    Json(args): Json<MergeMarks>,
) -> Result<Json<MutationResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .merge_marks(args.target_id, args.source_id, args.landmark_id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMarks {
    mark_ids: Vec<MarkID>,
    landmark_id: MarkID,
}

pub async fn delete_marks(
    Instance(rt): Instance,
    Json(args): Json<DeleteMarks>,
) -> Result<Json<MutationResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .delete_marks(&args.mark_ids, args.landmark_id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReparentMark {
    mark_id: MarkID,
    new_parent_id: MarkID,
    after_id: Option<MarkID>,
    landmark_id: MarkID,
}

pub async fn reparent_mark(
    Instance(rt): Instance,
    Json(args): Json<ReparentMark>,
) -> Result<Json<MutationResult>, ApiError> {
    Ok(Json(
        rt.editor()
            .reparent_mark(
                args.mark_id,
                args.new_parent_id,
                args.after_id,
                args.landmark_id,
            )
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Landmark {
    landmark_id: MarkID,
}

pub async fn undo(
    Instance(rt): Instance,
    Json(args): Json<Landmark>,
) -> Result<Json<Option<UndoRedoResult>>, ApiError> {
    Ok(Json(rt.editor().undo(args.landmark_id).await?))
}

pub async fn redo(
    Instance(rt): Instance,
    Json(args): Json<Landmark>,
) -> Result<Json<Option<UndoRedoResult>>, ApiError> {
    Ok(Json(rt.editor().redo(args.landmark_id).await?))
}
