use domain::chart::{Chart, ChartID};

use crate::collections::ChartRow;
use crate::error::StoreResult;
use crate::store::Store;

/// List all charts.
pub async fn list(store: &Store) -> StoreResult<Vec<Chart>> {
    let rows = store.db().all::<ChartRow>().await?;
    Ok(rows.into_iter().map(Chart::from).collect())
}

/// Fetch a single chart by ID.
pub async fn get(store: &Store, id: ChartID) -> StoreResult<Option<Chart>> {
    let row = store.db().get::<ChartRow>(&id).await?;
    Ok(row.map(Chart::from))
}
