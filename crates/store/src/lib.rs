pub use sea_orm::DatabaseConnection;

pub mod error;
pub use error::{StoreError, StoreResult};

mod macros;

/// Database migration entry point.
///
/// Two phases:
///
/// 1. **DDL** via refinery — SQL files in `migrations/sql/`, embedded at
///    compile time, tracked in `refinery_schema_history`. A custom
///    [`turso_adapter::TursoConn`] bridges refinery's async traits to the
///    [`turso::Connection`].
/// 2. **Seeds** via [`seeds::run`] — runtime Rust seeders (charts, mock
///    documents) that probe for stable UUIDs and skip themselves on
///    subsequent boots.
mod migrations {
    mod seed_charts;
    #[cfg(feature = "mock")]
    mod seed_documents;
    mod seeds;
    mod turso_adapter;

    use turso::Connection;

    use crate::StoreResult;

    mod embedded {
        refinery::embed_migrations!("src/migrations/sql");
    }

    pub async fn run(conn: &Connection) -> StoreResult<()> {
        let mut adapter = turso_adapter::TursoConn(conn.clone());
        let report = embedded::migrations::runner()
            .run_async(&mut adapter)
            .await?;
        tracing::info!(
            applied = report.applied_migrations().len(),
            "refinery: ddl migrations applied"
        );

        seeds::run(conn).await?;
        Ok(())
    }
}

pub mod tools;
pub use tools::{checkpoint, open, reset};

pub mod entities {
    mod chart_orm;
    mod device_orm;
    mod mark_orm;
    mod sync_log_orm;
    mod terrain_orm;

    pub use chart_orm::{
        ActiveModel as ActiveChart, Column as ChartColumn, Entity as Charts, Model as ChartModel,
    };
    pub use device_orm::{
        ActiveModel as ActiveDevice, Column as DeviceColumn, Entity as Devices,
        Model as DeviceModel,
    };
    pub use mark_orm::{
        ActiveModel as ActiveMark, Column as MarkColumn, Entity as Marks, Model as MarkModel,
    };
    pub use sync_log_orm::{
        ActiveModel as ActiveSyncLog, Column as SyncLogColumn, Entity as SyncLog,
        Model as SyncLogModel,
    };
    pub use terrain_orm::{
        ActiveModel as ActiveTerrain, Column as TerrainColumn, Entity as Terrains,
        Model as TerrainModel, kind_to_string as terrain_kind_to_string,
        string_to_kind as terrain_string_to_kind,
    };
}

pub mod query {
    pub mod chart;
    pub mod device;
    pub mod mark;
    pub mod sync_log;
    pub mod terrain;
}

#[cfg(feature = "test-utils")]
pub mod test_utils;

/// Built-in [`Chart`][C] types that provide common schemas out-of-the-box for
/// Waypoints.
///
/// These built-ins serve as:
/// - Ready-to-use types for common use-cases.
/// - Templates that users can build upon to create their own custom charts.
/// - Defaults that can be reset if users break their schemas.
///
/// The built-in system follows these principles:
/// - Each built-in chart has a fixed [`ChartID`][id] for consistency.
/// - Uses can reset to these original schemas at any time.
/// - Built-ins cover common content management and productivity needs.
///
/// [C]: domain::Chart
/// [id]: domain::ChartID
pub mod builtin {
    mod _utils;
    use _utils::{status_multi_select, tags_feature};

    pub mod book;
    pub mod bookmark;
    pub mod campaign;
    pub mod journal;
    pub mod meeting;
    pub mod note;
    pub mod objective;
    pub mod page;
    pub mod person;
}
