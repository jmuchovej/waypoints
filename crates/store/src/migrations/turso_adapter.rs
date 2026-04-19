//! Refinery [`AsyncTransaction`] + [`AsyncQuery`] adapter for [`turso::Connection`].
//!
//! Refinery doesn't ship a turso backend (supported: postgres, mysql, rusqlite,
//! tiberius). This thin newtype wraps a [`turso::Connection`] and implements
//! refinery's async traits so `Runner::run_async` can drive turso.
//!
//! `turso::Connection` is `Send + Sync + Clone` (cheap Arc clone), so the
//! adapter is also `Send + Sync` as required by `#[async_trait]`.

use async_trait::async_trait;
use refinery_core::Migration;
use refinery_core::traits::r#async::{AsyncMigrate, AsyncQuery, AsyncTransaction};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use turso::Connection;

use crate::error::StoreError;

pub struct TursoConn(pub Connection);

#[async_trait]
impl AsyncTransaction for TursoConn {
    type Error = StoreError;

    async fn execute<'a, T: Iterator<Item = &'a str> + Send>(
        &mut self,
        queries: T,
    ) -> Result<usize, Self::Error> {
        self.0.execute("BEGIN", ()).await?;

        let result: Result<usize, StoreError> = async {
            let mut count = 0usize;
            for query in queries {
                // execute_batch handles SQL files containing multiple
                // semicolon-separated statements.
                self.0.execute_batch(query).await?;
                count += 1;
            }
            Ok(count)
        }
        .await;

        match result {
            Ok(count) => {
                self.0.execute("COMMIT", ()).await?;
                Ok(count)
            }
            Err(e) => {
                // Best effort rollback; surface the original error.
                let _ = self.0.execute("ROLLBACK", ()).await;
                Err(e)
            }
        }
    }
}

#[async_trait]
impl AsyncQuery<Vec<Migration>> for TursoConn {
    async fn query(&mut self, query: &str) -> Result<Vec<Migration>, Self::Error> {
        let mut rows = self.0.query(query, ()).await?;
        let mut applied = Vec::new();
        while let Some(row) = rows.next().await? {
            let version: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let applied_on_raw: String = row.get(2)?;
            let checksum_raw: String = row.get(3)?;

            let applied_on = OffsetDateTime::parse(&applied_on_raw, &Rfc3339).map_err(|e| {
                StoreError::Custom(format!("invalid applied_on in migration history: {e}"))
            })?;
            let checksum: u64 = checksum_raw.parse().map_err(|e| {
                StoreError::Custom(format!("invalid checksum in migration history: {e}"))
            })?;

            applied.push(Migration::applied(version, name, applied_on, checksum));
        }
        Ok(applied)
    }
}

impl AsyncMigrate for TursoConn {}
