use domain::chart::{Chart, ChartID};
use domain::mark::{Landmark, Mark, MarkData};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait};
use store::entities::{ActiveMark, Charts};
use tauri::State;

use crate::mark::{MarkError, MarkResult};
use crate::territory::TerritoryManager;

#[tauri::command]
#[specta::specta]
pub async fn chart_list(mgr: State<'_, TerritoryManager>) -> MarkResult<Vec<Chart>> {
    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let models = Charts::find().all(db).await?;
    Ok(models.into_iter().map(Chart::from).collect())
}

#[tauri::command]
#[specta::specta]
pub async fn chart_get(
    mgr: State<'_, TerritoryManager>,
    chart_id: ChartID,
) -> MarkResult<Option<Chart>> {
    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let model = Charts::find_by_id(chart_id.0).one(db).await?;
    Ok(model.map(Chart::from))
}

#[tauri::command]
#[specta::specta]
pub async fn landmark_create(
    mgr: State<'_, TerritoryManager>,
    chart_id: ChartID,
) -> MarkResult<Mark> {
    let db_guard = mgr.active_db.lock().await;
    let db = db_guard.as_ref().ok_or(MarkError::NoActiveTerritory)?;

    let landmark = Landmark::builder().with_chart(chart_id).build();
    let mark_data = MarkData::Landmark(landmark);
    let id = uuid::Uuid::new_v4();

    let data_json =
        serde_json::to_value(&mark_data).map_err(|e| MarkError::Serialization(e.to_string()))?;

    let active = ActiveMark {
        id: Set(id),
        mark: Set(mark_data.type_name()),
        data: Set(data_json),
        extra: Set(None),
        ..Default::default()
    };
    let model = active.insert(db).await?;

    let mark = Mark::try_from(model)?;
    tracing::info!(%id, type_name = mark.type_name(), "landmark_create: done");
    Ok(mark)
}
