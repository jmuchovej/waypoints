use bedrock_core::{Collection, Field};

use crate::error::BedrockError;
use crate::{ddl, value};

/// A bedrock database: a turso connection plus collection-aware operations.
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
        let params = value::to_params(&spec, &entity.to_row()?)?;
        self.conn.execute(ddl::insert(&spec, false), params).await?;
        Ok(())
    }

    /// Insert many entities atomically — all or nothing.
    pub async fn insert_all<C: Collection>(&self, entities: &[C]) -> Result<(), BedrockError> {
        let spec = C::spec();
        let sql = ddl::insert(&spec, false);
        self.conn.execute("BEGIN IMMEDIATE", ()).await?;
        for entity in entities {
            // Any failure between BEGIN and COMMIT must ROLLBACK before returning,
            // otherwise the connection is left mid-transaction.
            let step = async {
                let params = value::to_params(&spec, &entity.to_row()?)?;
                self.conn.execute(&sql, params).await?;
                Ok::<(), BedrockError>(())
            }
            .await;
            if let Err(error) = step {
                let _ = self.conn.execute("ROLLBACK", ()).await;
                return Err(error);
            }
        }
        self.conn.execute("COMMIT", ()).await?;
        Ok(())
    }

    /// Insert or replace one entity, keyed on the primary key.
    pub async fn upsert<C: Collection>(&self, entity: &C) -> Result<(), BedrockError> {
        let spec = C::spec();
        let params = value::to_params(&spec, &entity.to_row()?)?;
        self.conn.execute(ddl::insert(&spec, true), params).await?;
        Ok(())
    }

    /// Update one entity in place, matched by its primary key. Returns whether a
    /// row was affected.
    pub async fn update<C: Collection>(&self, entity: &C) -> Result<bool, BedrockError> {
        let spec = C::spec();
        let row = entity.to_row()?;
        let id_index = spec
            .fields
            .iter()
            .position(|f| f.name == spec.id.field)
            .expect("the derive puts the id among the fields");
        let mut params = value::to_params(&spec, &row)?;
        params.push(value::to_turso(&row[id_index])?);
        let affected = self.conn.execute(ddl::update(&spec), params).await?;
        Ok(affected > 0)
    }

    /// Fetch one entity by primary key.
    pub async fn get<C: Collection>(&self, id: &C::Id) -> Result<Option<C>, BedrockError> {
        let spec = C::spec();
        let is_document: Vec<bool> = spec.fields.iter().map(|f| f.is_document()).collect();
        let param = value::to_turso(&id.encode()?)?;
        let mut rows = self
            .conn
            .query(ddl::select_by_id(&spec), vec![param])
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(C::from_row(value::read_row(&row, &is_document)?)?)),
            None => Ok(None),
        }
    }

    /// Delete one entity by primary key. Returns whether a row was removed.
    pub async fn delete<C: Collection>(&self, id: &C::Id) -> Result<bool, BedrockError> {
        let spec = C::spec();
        let param = value::to_turso(&id.encode()?)?;
        let affected = self
            .conn
            .execute(ddl::delete_by_id(&spec), vec![param])
            .await?;
        Ok(affected > 0)
    }

    /// Count the rows in the collection.
    pub async fn count<C: Collection>(&self) -> Result<u64, BedrockError> {
        let spec = C::spec();
        let mut rows = self.conn.query(ddl::count(&spec), ()).await?;
        let count = match rows.next().await? {
            Some(row) => match row.get_value(0)? {
                turso::Value::Integer(n) => n.max(0) as u64,
                _ => 0,
            },
            None => 0,
        };
        Ok(count)
    }

    /// Read every entity in the collection.
    pub async fn all<C: Collection>(&self) -> Result<Vec<C>, BedrockError> {
        let spec = C::spec();
        let is_document: Vec<bool> = spec.fields.iter().map(|f| f.is_document()).collect();
        let mut rows = self.conn.query(ddl::select_all(&spec), ()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(C::from_row(value::read_row(&row, &is_document)?)?);
        }
        Ok(out)
    }
}
