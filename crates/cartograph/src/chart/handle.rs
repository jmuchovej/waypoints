use crate::{
    Database,
    mark::{MarkError, MarkResult},
};
use domain::{Chart, ChartID};

pub struct ChartHandle<'a> {
    pub(crate) db: &'a Database,
}

impl ChartHandle<'_> {
    async fn db(&self) -> MarkResult<store::Store> {
        self.db
            .lock()
            .await
            .clone()
            .ok_or(MarkError::NoActiveTerritory)
    }

    pub async fn list(&self) -> MarkResult<Vec<Chart>> {
        let db = self.db().await?;
        Ok(store::query::chart::list(&db).await?)
    }

    pub async fn get(&self, chart_id: ChartID) -> MarkResult<Option<Chart>> {
        let db = self.db().await?;
        Ok(store::query::chart::get(&db, chart_id).await?)
    }
}
