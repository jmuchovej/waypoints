use chrono::NaiveDateTime;
use domain::mark::{Mark, MarkData, MarkID};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter,
};
use uuid::Uuid;

use crate::entities::{ActiveMark, MarkColumn, Marks};
use crate::error::{StoreError, StoreResult};

/// Fetch a single non-deleted mark by ID.
pub async fn get(db: &DatabaseConnection, id: MarkID) -> StoreResult<Option<Mark>> {
    let model = Marks::find_by_id(id.inner())
        .filter(MarkColumn::DeletedAt.is_null())
        .one(db)
        .await?;
    model.map(Mark::try_from).transpose().map_err(Into::into)
}

/// List all non-deleted landmark marks.
pub async fn list_landmarks(db: &DatabaseConnection) -> StoreResult<Vec<Mark>> {
    let models = Marks::find()
        .filter(MarkColumn::Mark.starts_with("landmark."))
        .filter(MarkColumn::DeletedAt.is_null())
        .all(db)
        .await?;
    models
        .into_iter()
        .map(Mark::try_from)
        .collect::<StoreResult<_>>()
        .map_err(Into::into)
}

/// Batch-fetch non-deleted marks by IDs (arbitrary order).
pub async fn get_many(db: &DatabaseConnection, ids: &[Uuid]) -> StoreResult<Vec<Mark>> {
    let models = Marks::find()
        .filter(MarkColumn::Id.is_in(ids.to_vec()))
        .filter(MarkColumn::DeletedAt.is_null())
        .all(db)
        .await?;
    models
        .into_iter()
        .map(Mark::try_from)
        .collect::<StoreResult<_>>()
        .map_err(Into::into)
}

/// Insert a new mark.
pub async fn insert(
    db: &DatabaseConnection,
    id: MarkID,
    data: &MarkData,
    created_at: NaiveDateTime,
) -> StoreResult<()> {
    let data_json = serde_json::to_value(data)?;
    let active = ActiveMark {
        id: Set(id.inner()),
        mark: Set(data.type_name()),
        data: Set(data_json),
        extra: Set(None),
        created_at: Set(created_at),
        ..Default::default()
    };
    active.insert(db).await?;
    Ok(())
}

/// Update the data column. Errors with NotFound if missing/deleted.
pub async fn update_data(db: &DatabaseConnection, id: MarkID, data: &MarkData) -> StoreResult<()> {
    let model = Marks::find_by_id(id.inner())
        .filter(MarkColumn::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or(StoreError::MarkNotFound(id))?;

    let data_json = serde_json::to_value(data)?;
    let mut active: ActiveMark = model.into_active_model();
    active.data = Set(data_json);
    active.update(db).await?;
    Ok(())
}

/// Soft-delete (set deleted_at). Errors with NotFound if missing.
pub async fn soft_delete(db: &DatabaseConnection, id: MarkID) -> StoreResult<()> {
    let model = Marks::find_by_id(id.inner())
        .filter(MarkColumn::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or(StoreError::MarkNotFound(id))?;

    let now = chrono::Local::now().naive_utc();
    let mut active: ActiveMark = model.into_active_model();
    active.deleted_at = Set(Some(now));
    active.update(db).await?;
    Ok(())
}

/// Restore a soft-deleted mark (clear deleted_at). Errors if not found or not deleted.
pub async fn restore(db: &DatabaseConnection, id: MarkID) -> StoreResult<()> {
    let model = Marks::find_by_id(id.inner())
        .one(db)
        .await?
        .ok_or(StoreError::MarkNotFound(id))?;

    let mut active: ActiveMark = model.into_active_model();
    active.deleted_at = Set(None);
    active.update(db).await?;
    Ok(())
}

/// Hard-delete (actually deletes). Doesn't error if the record is absent.
pub async fn hard_delete(db: &DatabaseConnection, id: MarkID) -> StoreResult<()> {
    let model = Marks::find_by_id(id.inner())
        .one(db)
        .await?
        .ok_or(StoreError::MarkNotFound(id))?;

    let active: ActiveMark = model.into_active_model();
    active.delete(db).await?;
    Ok(())
}
