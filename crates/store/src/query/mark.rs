use chrono::NaiveDateTime;
use domain::chart::ChartID;
use domain::mark::{Mark, MarkData, MarkID};
use domain::sync::{DeviceID, HLC};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, sea_query::Expr,
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

/// List non-deleted marks, optionally filtered by type prefix and/or chart IDs.
///
/// When `prefix` is `None`, returns all non-deleted marks.
/// When `prefix` is `Some("landmark")`, returns marks whose type starts with `"landmark."`.
/// When `chart_ids` is provided, further filters to marks matching those chart types.
pub async fn list(
    db: &DatabaseConnection,
    prefix: Option<&str>,
    chart_ids: Option<&[ChartID]>,
) -> StoreResult<Vec<Mark>> {
    let mut query = Marks::find().filter(MarkColumn::DeletedAt.is_null());

    if let Some(ids) = chart_ids {
        // Filter by specific chart types: mark column is "landmark.{chart.name()}"
        let condition = ids.iter().fold(Condition::any(), |cond, id| {
            cond.add(MarkColumn::Mark.starts_with(&format!("landmark.{}", id.name())))
        });
        query = query.filter(condition);
    } else if let Some(p) = prefix {
        query = query.filter(MarkColumn::Mark.starts_with(&format!("{p}.")));
    }

    let models = query.all(db).await?;
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

/// Insert a new mark, returning the created [`Mark`].
pub async fn insert(
    db: &DatabaseConnection,
    id: MarkID,
    data: &MarkData,
    created_at: NaiveDateTime,
) -> StoreResult<Mark> {
    let data_json = serde_json::to_value(data)?;
    let active = ActiveMark {
        id: Set(id.inner()),
        mark: Set(data.type_name()),
        data: Set(data_json),
        extra: Set(None),
        created_at: Set(created_at),
        ..Default::default()
    };
    let model = active.insert(db).await?;
    Mark::try_from(model)
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

// ── Sync row clocks (#130) ──────────────────────────────────────────────

/// Read the sync row clock for a mark. `None` means the row is missing
/// or has never been touched by sync (`updated_hlc = 0`) — either way,
/// any incoming op dominates it. Soft-deleted rows keep their clock:
/// deletion is itself a synced write.
pub async fn get_row_clock(
    db: &DatabaseConnection,
    id: MarkID,
) -> StoreResult<Option<(HLC, DeviceID)>> {
    let model = Marks::find_by_id(id.inner()).one(db).await?;
    Ok(
        model.and_then(|m| match (m.updated_hlc, m.updated_by_device) {
            (0, _) | (_, None) => None,
            (hlc, Some(device)) => Some((HLC(hlc), DeviceID::from(device))),
        }),
    )
}

/// Set the sync row clock for a mark (including soft-deleted ones). Uses
/// a direct column update so it never triggers entity save hooks or
/// touches `updated_at`.
pub async fn set_row_clock(
    db: &DatabaseConnection,
    id: MarkID,
    hlc: HLC,
    device: DeviceID,
) -> StoreResult<()> {
    let result = Marks::update_many()
        .col_expr(MarkColumn::UpdatedHlc, Expr::value(hlc.inner()))
        .col_expr(MarkColumn::UpdatedByDevice, Expr::value(device.inner()))
        .filter(MarkColumn::Id.eq(id.inner()))
        .exec(db)
        .await?;
    if result.rows_affected == 0 {
        return Err(StoreError::MarkNotFound(id));
    }
    Ok(())
}
