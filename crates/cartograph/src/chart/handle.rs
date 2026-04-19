use domain::chart::{Chart, ChartID};
use domain::mark::{Landmark, Mark, MarkData, MarkID};
use uuid::Uuid;

use crate::Database;

use crate::mark::{MarkError, MarkResult};

pub struct ChartHandle<'a> {
    pub(crate) db: &'a Database,
}

impl ChartHandle<'_> {
    async fn db(&self) -> MarkResult<store::DatabaseConnection> {
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

    pub async fn create_landmark(&self, chart_id: ChartID) -> MarkResult<Mark> {
        let db = self.db().await?;

        let landmark = Landmark::builder().with_chart(chart_id).build();
        let mark_data = MarkData::Landmark(landmark);
        let id = MarkID::from(Uuid::new_v4());
        let created_at = chrono::Local::now().naive_utc();

        let mark = store::query::mark::insert(&db, id, &mark_data, created_at).await?;

        tracing::info!(%id, type_name = mark.type_name(), "landmark_create: done");
        Ok(mark)
    }
}
