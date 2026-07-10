use domain::chart::{Chart, ChartID};

use crate::error::StoreResult;
use crate::store::Store;

/// List all charts.
pub async fn list(store: &Store) -> StoreResult<Vec<Chart>> {
    store.db().all::<Chart>().await.map_err(Into::into)
}

/// Fetch a single chart by ID.
pub async fn get(store: &Store, id: ChartID) -> StoreResult<Option<Chart>> {
    store.db().get::<Chart>(&id).await.map_err(Into::into)
}
