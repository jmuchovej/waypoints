use bedrock_core::{Collection, CollectionSpec, Field, Query, StoredValue, ViewSpec};

use crate::error::BedrockError;
use crate::{ddl, migrate, value};

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
            .experimental_materialized_views(true)
            .build()
            .await?;
        let conn = engine.connect()?;
        Ok(Self { conn })
    }

    /// Create the collection's table if it is new, or migrate it to the current
    /// spec if it already exists — safe-additive changes (new nullable columns,
    /// index add/drop) are applied; destructive ones are refused. Idempotent.
    pub async fn ensure_collection<C: Collection>(&self) -> Result<(), BedrockError> {
        self.ensure_spec(&C::spec()).await
    }

    /// The runtime-spec form of [`ensure_collection`](Self::ensure_collection),
    /// for collections defined as data rather than through the derive.
    pub async fn ensure_spec(&self, spec: &CollectionSpec) -> Result<(), BedrockError> {
        spec.validate()?;
        self.conn.execute(ddl::create_meta_table(), ()).await?;
        match self.stored_spec(&spec.name).await? {
            None => {
                self.conn.execute(ddl::create_table(spec), ()).await?;
                for index in &spec.indexes {
                    self.conn
                        .execute(ddl::create_index(spec, index), ())
                        .await?;
                }
                self.record_spec(spec).await?;
            }
            Some(ref stored) if stored == spec => {}
            Some(ref stored) => {
                let statements = migrate::plan(stored, spec)?;
                self.conn.execute("BEGIN IMMEDIATE", ()).await?;
                let applied = async {
                    for statement in &statements {
                        self.conn.execute(statement, ()).await?;
                    }
                    self.record_spec(spec).await?;
                    Ok::<(), BedrockError>(())
                }
                .await;
                if let Err(error) = applied {
                    let _ = self.conn.execute("ROLLBACK", ()).await;
                    return Err(error);
                }
                self.conn.execute("COMMIT", ()).await?;
            }
        }
        Ok(())
    }

    /// The last-recorded spec for a collection, if any.
    async fn stored_spec(&self, name: &str) -> Result<Option<CollectionSpec>, BedrockError> {
        let mut rows = self
            .conn
            .query(ddl::read_spec(), vec![turso::Value::Text(name.to_owned())])
            .await?;
        match rows.next().await? {
            Some(row) => match row.get_value(0)? {
                turso::Value::Text(json) => Ok(Some(serde_json::from_str(&json)?)),
                _ => Ok(None),
            },
            None => Ok(None),
        }
    }

    /// Upsert a collection's spec into the meta table.
    async fn record_spec(&self, spec: &CollectionSpec) -> Result<(), BedrockError> {
        let json = serde_json::to_string(spec)?;
        self.conn
            .execute(
                ddl::store_spec(),
                vec![
                    turso::Value::Text(spec.name.clone()),
                    turso::Value::Text(json),
                ],
            )
            .await?;
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

    /// Declare (or replace) a materialized view over collection `C`.
    ///
    /// Idempotent: any existing view of the same name is dropped and recreated,
    /// backfilling from the collection's current rows. This is recompute-based —
    /// the pinned engine's incremental view maintenance is unreliable under
    /// explicit write transactions, so a view is refreshed by re-declaring it,
    /// not kept live per-write. The view carries no secondary indexes for the
    /// same reason (they desync on base updates).
    pub async fn declare_view<C: Collection>(&self, view: &ViewSpec) -> Result<(), BedrockError> {
        let spec = C::spec();
        self.conn.execute(ddl::drop_view(&view.name), ()).await?;
        self.conn.execute(ddl::create_view(&spec, view), ()).await?;
        Ok(())
    }

    /// Drop a materialized view if it exists.
    pub async fn drop_view(&self, name: &str) -> Result<(), BedrockError> {
        self.conn.execute(ddl::drop_view(name), ()).await?;
        Ok(())
    }

    /// Read every row of a view: `[id, projected columns…]` per row, in view order.
    pub async fn view_rows(&self, name: &str) -> Result<Vec<Vec<StoredValue>>, BedrockError> {
        self.projected_rows(ddl::select_view(name), &Query::new())
            .await
    }

    /// Query a collection: filter, sort, and limit over its base columns, reading
    /// back full entities.
    pub async fn query<C: Collection>(&self, query: &Query) -> Result<Vec<C>, BedrockError> {
        let spec = C::spec();
        let is_document: Vec<bool> = spec.fields.iter().map(|f| f.is_document()).collect();
        let (suffix, params) = crate::query::suffix(query);
        let sql = format!("{}{}", ddl::select_all(&spec), suffix);
        let mut rows = self.conn.query(sql, params).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(C::from_row(value::read_row(&row, &is_document)?)?);
        }
        Ok(out)
    }

    /// Query a materialized view: filter/sort/limit over its projected columns,
    /// returning projected rows (`[id, columns…]`).
    pub async fn query_view(
        &self,
        name: &str,
        query: &Query,
    ) -> Result<Vec<Vec<StoredValue>>, BedrockError> {
        self.projected_rows(ddl::select_view(name), query).await
    }

    /// Run a `SELECT` head + query suffix and read each row as scalar values.
    async fn projected_rows(
        &self,
        head: String,
        query: &Query,
    ) -> Result<Vec<Vec<StoredValue>>, BedrockError> {
        let (suffix, params) = crate::query::suffix(query);
        let mut rows = self.conn.query(format!("{head}{suffix}"), params).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let count = row.column_count();
            let mut projected = Vec::with_capacity(count);
            for index in 0..count {
                projected.push(value::from_turso_scalar(row.get_value(index)?)?);
            }
            out.push(projected);
        }
        Ok(out)
    }
}
