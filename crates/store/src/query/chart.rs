use domain::chart::{Chart, ChartID};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::entities::Charts;
use crate::error::{StoreError, StoreResult};

/// List all charts.
pub async fn list(db: &DatabaseConnection) -> StoreResult<Vec<Chart>> {
    let models = Charts::find().all(db).await?;
    Ok(models.into_iter().map(Chart::from).collect())
}

/// Fetch a single chart by ID.
pub async fn get(db: &DatabaseConnection, id: ChartID) -> StoreResult<Option<Chart>> {
    let model = Charts::find_by_id(id.0.clone()).one(db).await?;
    Ok(model.map(Chart::from))
}
