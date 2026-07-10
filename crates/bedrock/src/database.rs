use bedrock_core::Collection;

use crate::error::BedrockError;
use crate::{ddl, value};

/// A bedrock database: a turso connection plus collection-aware operations.
///
/// Deliberately the naive-but-correct core: open, create a table, insert,
/// read all.
pub struct Database {
    // The connection keeps the engine alive on its own: turso_core::Connection
    // holds `db: Arc<Database>`, so no handle to `turso::Database` is needed.
    conn: turso::Connection,
}

impl Database {
    /// Open (or create) a database at `path`. Use `":memory:"` for an ephemeral one.
    ///
    /// Custom column types (uuid/jsonb/boolean/date/time/timestamp) are enabled
    /// here; STRICT is applied per-table in the DDL.
    pub async fn open(path: &str) -> Result<Self, BedrockError> {
        let engine = turso::Builder::new_local(path)
            .experimental_custom_types(true)
            .build()
            .await?;
        let conn = engine.connect()?;
        Ok(Self { conn })
    }

    /// Create the collection's table if it does not already exist.
    pub async fn ensure_collection<C: Collection>(&self) -> Result<(), BedrockError> {
        let spec = C::spec();
        spec.validate()?;
        self.conn.execute(ddl::create_table(&spec), ()).await?;
        Ok(())
    }

    /// Insert one entity.
    pub async fn insert<C: Collection>(&self, entity: &C) -> Result<(), BedrockError> {
        let spec = C::spec();
        let row = entity.to_row()?;
        let mut params = Vec::with_capacity(row.len());
        for (field, value) in spec.fields.iter().zip(row) {
            value::reject_submillis(field.kind, &value, &field.name)?;
            params.push(value::to_turso(value)?);
        }
        self.conn.execute(ddl::insert(&spec), params).await?;
        Ok(())
    }

    /// Read every entity in the collection.
    pub async fn all<C: Collection>(&self) -> Result<Vec<C>, BedrockError> {
        let spec = C::spec();
        let is_document: Vec<bool> = spec.fields.iter().map(|f| f.is_document()).collect();

        let mut rows = self.conn.query(ddl::select_all(&spec), ()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let mut stored = Vec::with_capacity(is_document.len());
            for (index, &is_doc) in is_document.iter().enumerate() {
                stored.push(value::from_turso(row.get_value(index)?, is_doc)?);
            }
            out.push(C::from_row(stored)?);
        }
        Ok(out)
    }
}
