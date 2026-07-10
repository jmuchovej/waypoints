use bedrock_core::{
    Collection, CollectionSpec, Field, Filter, Query, StoredValue, Update, ViewSpec,
};
use turso::Connection;

use crate::config::Config;
use crate::durability::{self, FlushMode};
use crate::error::BedrockError;
use crate::explain::{self, QueryPlan};
use crate::pool::Pool;
use crate::tx::{ReadTx, WriteTx};
use crate::{ddl, migrate, value};

/// A bedrock database: a pooled turso engine plus collection-aware operations.
///
/// Reads borrow a pooled read connection; writes serialize on the single write
/// connection. The pool shares one engine handle, so every connection sees the
/// same data (including for `:memory:`).
pub struct Database {
    pool: Pool,
}

impl Database {
    /// Open (or create) a database at `path` with default [`Config`]. Use
    /// `":memory:"` for an ephemeral one.
    pub async fn open(path: &str) -> Result<Self, BedrockError> {
        Self::open_with(path, Config::default()).await
    }

    /// Open (or create) a database at `path` with an explicit [`Config`]:
    /// read-pool size, per-connection pragmas, and the enforced schema version.
    ///
    /// Custom column types (uuid/jsonb/boolean/date/time/timestamp) and
    /// materialized views are enabled here; STRICT is applied per-table in the
    /// DDL. The `bedrock_collections` and `bedrock_views` bookkeeping tables are
    /// created once, so ordinary reads never issue DDL.
    pub async fn open_with(path: &str, config: Config) -> Result<Self, BedrockError> {
        // Determine freshness before building: `build` creates the file.
        let is_new = path == ":memory:" || !std::path::Path::new(path).exists();
        let engine = turso::Builder::new_local(path)
            .experimental_custom_types(true)
            .experimental_materialized_views(true)
            .build()
            .await?;
        let pool = Pool::open(path, engine, config, is_new).await?;
        {
            let conn = pool.write().await?;
            conn.execute(ddl::create_meta_table(), ()).await?;
            conn.execute(ddl::create_views_table(), ()).await?;
        }
        Ok(Self { pool })
    }

    /// Close the pool, dropping every connection. Subsequent operations fail
    /// with [`BedrockError::Closed`].
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Begin an explicit read-only transaction (`BEGIN`) on a pooled read
    /// connection, for multi-statement reads that need a stable snapshot.
    pub async fn begin_read(&self) -> Result<ReadTx, BedrockError> {
        ReadTx::begin(self.pool.read().await?).await
    }

    /// Begin an explicit read-write transaction (`BEGIN IMMEDIATE`) on the write
    /// connection. Supports nested [`savepoint`](WriteTx::savepoint)s.
    pub async fn begin_write(&self) -> Result<WriteTx, BedrockError> {
        WriteTx::begin(self.pool.write().await?).await
    }

    /// Flush durable state to disk with the default mode
    /// ([`FlushMode::Passive`]).
    pub async fn flush(&self) -> Result<(), BedrockError> {
        self.flush_with(FlushMode::default()).await
    }

    /// Flush durable state to disk with an explicit [`FlushMode`].
    pub async fn flush_with(&self, mode: FlushMode) -> Result<(), BedrockError> {
        let conn = self.pool.write().await?;
        durability::flush(&conn, mode).await
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
        let conn = self.pool.write().await?;
        match stored_spec(&conn, &spec.name).await? {
            None => {
                conn.execute(ddl::create_table(spec), ()).await?;
                for index in &spec.indexes {
                    conn.execute(ddl::create_index(spec, index), ()).await?;
                }
                record_spec(&conn, spec).await?;
            }
            Some(ref stored) if stored == spec => {}
            Some(ref stored) => {
                let statements = migrate::plan(stored, spec)?;
                conn.execute("BEGIN IMMEDIATE", ()).await?;
                let applied = async {
                    for statement in &statements {
                        conn.execute(statement, ()).await?;
                    }
                    record_spec(&conn, spec).await?;
                    Ok::<(), BedrockError>(())
                }
                .await;
                if let Err(error) = applied {
                    let _ = conn.execute("ROLLBACK", ()).await;
                    return Err(error);
                }
                conn.execute("COMMIT", ()).await?;
            }
        }
        Ok(())
    }

    /// Insert one entity.
    pub async fn insert<C: Collection>(&self, entity: &C) -> Result<(), BedrockError> {
        let spec = C::spec();
        let params = value::to_params(&spec, &entity.to_row()?)?;
        let conn = self.pool.write().await?;
        conn.execute(ddl::insert(&spec, false), params).await?;
        Ok(())
    }

    /// Insert many entities atomically — all or nothing.
    pub async fn insert_all<C: Collection>(&self, entities: &[C]) -> Result<(), BedrockError> {
        let spec = C::spec();
        let sql = ddl::insert(&spec, false);
        let tx = self.begin_write().await?;
        for entity in entities {
            // Any failure drops `tx` before commit; the pool rolls the
            // connection back on its next acquisition.
            let params = value::to_params(&spec, &entity.to_row()?)?;
            tx.execute(&sql, params).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Insert or replace one entity, keyed on the primary key.
    pub async fn upsert<C: Collection>(&self, entity: &C) -> Result<(), BedrockError> {
        let spec = C::spec();
        let params = value::to_params(&spec, &entity.to_row()?)?;
        let conn = self.pool.write().await?;
        conn.execute(ddl::insert(&spec, true), params).await?;
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
        let conn = self.pool.write().await?;
        let affected = conn.execute(ddl::update(&spec), params).await?;
        Ok(affected > 0)
    }

    /// Apply a field-level update (`$set`/`$unset`) to every row of `C` matching
    /// `filter`, returning the number of rows affected. Columns are validated
    /// against the spec: `$set`/`$unset` on an unknown column, or `$unset` on a
    /// non-nullable column, is refused.
    pub async fn update_where<C: Collection>(
        &self,
        filter: &Filter,
        update: &Update,
    ) -> Result<u64, BedrockError> {
        if update.is_empty() {
            return Ok(0);
        }
        let spec = C::spec();
        for assignment in &update.set {
            if spec.field(&assignment.column).is_none() {
                return Err(BedrockError::UnknownColumn {
                    column: assignment.column.clone(),
                });
            }
        }
        for column in &update.unset {
            match spec.field(column) {
                None => {
                    return Err(BedrockError::UnknownColumn {
                        column: column.clone(),
                    });
                }
                Some(field) if !field.nullable => {
                    return Err(BedrockError::UnsetNonNullable {
                        column: column.clone(),
                    });
                }
                Some(_) => {}
            }
        }
        let (sql, params) = crate::query::update_statement(&spec.name, filter, update);
        let conn = self.pool.write().await?;
        Ok(conn.execute(sql, params).await?)
    }

    /// Fetch one entity by primary key.
    pub async fn get<C: Collection>(&self, id: &C::Id) -> Result<Option<C>, BedrockError> {
        let spec = C::spec();
        let is_document: Vec<bool> = spec.fields.iter().map(|f| f.is_document()).collect();
        let param = value::to_turso(&id.encode()?)?;
        let conn = self.pool.read().await?;
        let mut rows = conn.query(ddl::select_by_id(&spec), vec![param]).await?;
        match rows.next().await? {
            Some(row) => Ok(Some(C::from_row(value::read_row(&row, &is_document)?)?)),
            None => Ok(None),
        }
    }

    /// Delete one entity by primary key. Returns whether a row was removed.
    pub async fn delete<C: Collection>(&self, id: &C::Id) -> Result<bool, BedrockError> {
        let spec = C::spec();
        let param = value::to_turso(&id.encode()?)?;
        let conn = self.pool.write().await?;
        let affected = conn.execute(ddl::delete_by_id(&spec), vec![param]).await?;
        Ok(affected > 0)
    }

    /// Count the rows in the collection.
    pub async fn count<C: Collection>(&self) -> Result<u64, BedrockError> {
        let spec = C::spec();
        let conn = self.pool.read().await?;
        let mut rows = conn.query(ddl::count(&spec), ()).await?;
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
        let conn = self.pool.read().await?;
        let mut rows = conn.query(ddl::select_all(&spec), ()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(C::from_row(value::read_row(&row, &is_document)?)?);
        }
        Ok(out)
    }

    /// Declare (or replace) a materialized view over collection `C`, recording it
    /// so it can be listed and cascaded on collection drop.
    ///
    /// Idempotent by name: any existing view of the same name is dropped and
    /// recreated, backfilling from the collection's current rows. Recompute-based —
    /// the pinned engine's incremental view maintenance is unreliable under
    /// explicit write transactions, so a view is refreshed by re-declaring it,
    /// not kept live per-write, and carries no secondary indexes (they desync on
    /// base updates).
    pub async fn declare_view<C: Collection>(&self, view: &ViewSpec) -> Result<(), BedrockError> {
        let spec = C::spec();
        let conn = self.pool.write().await?;
        declare_view_on(&conn, &spec, view).await
    }

    /// Declare the view only if it differs from what's recorded — a no-op when
    /// the stored spec already matches, avoiding an unnecessary rebuild.
    pub async fn ensure_view<C: Collection>(&self, view: &ViewSpec) -> Result<(), BedrockError> {
        let conn = self.pool.write().await?;
        if stored_view(&conn, &view.name).await?.as_ref() == Some(view) {
            return Ok(());
        }
        let spec = C::spec();
        declare_view_on(&conn, &spec, view).await
    }

    /// Drop a materialized view (and its record) if it exists.
    pub async fn drop_view(&self, name: &str) -> Result<(), BedrockError> {
        let conn = self.pool.write().await?;
        drop_view_on(&conn, name).await
    }

    /// Every declared view's spec.
    pub async fn views(&self) -> Result<Vec<ViewSpec>, BedrockError> {
        let conn = self.pool.read().await?;
        let mut rows = conn.query(ddl::list_views(), ()).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            if let turso::Value::Text(json) = row.get_value(0)? {
                out.push(serde_json::from_str(&json)?);
            }
        }
        Ok(out)
    }

    /// Drop collection `C` — its table, every view declared over it (cascade),
    /// and both records. A no-op if the collection does not exist.
    pub async fn drop_collection<C: Collection>(&self) -> Result<(), BedrockError> {
        let name = C::spec().name;
        let conn = self.pool.write().await?;

        // Collect the collection's view names first (draining the query before
        // issuing further statements on the same connection), then drop each.
        let mut view_names = Vec::new();
        let mut rows = conn
            .query(
                ddl::views_for_collection(),
                vec![turso::Value::Text(name.clone())],
            )
            .await?;
        while let Some(row) = rows.next().await? {
            if let turso::Value::Text(view_name) = row.get_value(0)? {
                view_names.push(view_name);
            }
        }
        drop(rows);
        for view_name in view_names {
            drop_view_on(&conn, &view_name).await?;
        }

        conn.execute(ddl::drop_table(&name), ()).await?;
        conn.execute(ddl::delete_spec(), vec![turso::Value::Text(name)])
            .await?;
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
        let conn = self.pool.read().await?;
        let mut rows = conn.query(sql, params).await?;
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

    /// Explain a collection query: the SQL it lowers to, its bound parameters,
    /// and turso's `EXPLAIN QUERY PLAN` for it. The plan is produced by running
    /// the exact SQL `query::<C>` would run, so it doubles as proof the lowering
    /// is valid turso SQL.
    pub async fn explain_query<C: Collection>(
        &self,
        query: &Query,
    ) -> Result<QueryPlan, BedrockError> {
        let spec = C::spec();
        let (suffix, params) = crate::query::suffix(query);
        let sql = format!("{}{}", ddl::select_all(&spec), suffix);
        let conn = self.pool.read().await?;
        explain::plan(&conn, sql, params).await
    }

    /// Explain a materialized-view query — the [`explain_query`](Self::explain_query)
    /// analogue for [`query_view`](Self::query_view).
    pub async fn explain_view_query(
        &self,
        name: &str,
        query: &Query,
    ) -> Result<QueryPlan, BedrockError> {
        let (suffix, params) = crate::query::suffix(query);
        let sql = format!("{}{}", ddl::select_view(name), suffix);
        let conn = self.pool.read().await?;
        explain::plan(&conn, sql, params).await
    }

    /// Explain a field-level update — the [`explain_query`](Self::explain_query)
    /// analogue for [`update_where`](Self::update_where). Columns are not
    /// re-validated here; this reflects the SQL as it would run.
    pub async fn explain_update<C: Collection>(
        &self,
        filter: &Filter,
        update: &Update,
    ) -> Result<QueryPlan, BedrockError> {
        let spec = C::spec();
        let (sql, params) = crate::query::update_statement(&spec.name, filter, update);
        let conn = self.pool.read().await?;
        explain::plan(&conn, sql, params).await
    }

    /// Run a `SELECT` head + query suffix and read each row as scalar values.
    async fn projected_rows(
        &self,
        head: String,
        query: &Query,
    ) -> Result<Vec<Vec<StoredValue>>, BedrockError> {
        let (suffix, params) = crate::query::suffix(query);
        let conn = self.pool.read().await?;
        let mut rows = conn.query(format!("{head}{suffix}"), params).await?;
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

/// The last-recorded spec for a collection, if any.
async fn stored_spec(
    conn: &Connection,
    name: &str,
) -> Result<Option<CollectionSpec>, BedrockError> {
    let mut rows = conn
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
async fn record_spec(conn: &Connection, spec: &CollectionSpec) -> Result<(), BedrockError> {
    let json = serde_json::to_string(spec)?;
    conn.execute(
        ddl::store_spec(),
        vec![
            turso::Value::Text(spec.name.clone()),
            turso::Value::Text(json),
        ],
    )
    .await?;
    Ok(())
}

/// The recorded spec for a view, if any.
async fn stored_view(conn: &Connection, name: &str) -> Result<Option<ViewSpec>, BedrockError> {
    let mut rows = conn
        .query(ddl::read_view(), vec![turso::Value::Text(name.to_owned())])
        .await?;
    match rows.next().await? {
        Some(row) => match row.get_value(0)? {
            turso::Value::Text(json) => Ok(Some(serde_json::from_str(&json)?)),
            _ => Ok(None),
        },
        None => Ok(None),
    }
}

/// Upsert a view's spec into the meta table.
async fn record_view(
    conn: &Connection,
    collection: &str,
    view: &ViewSpec,
) -> Result<(), BedrockError> {
    let json = serde_json::to_string(view)?;
    conn.execute(
        ddl::store_view(),
        vec![
            turso::Value::Text(view.name.clone()),
            turso::Value::Text(collection.to_owned()),
            turso::Value::Text(json),
        ],
    )
    .await?;
    Ok(())
}

/// Drop and recreate a view on `conn`, then record it.
async fn declare_view_on(
    conn: &Connection,
    spec: &CollectionSpec,
    view: &ViewSpec,
) -> Result<(), BedrockError> {
    conn.execute(ddl::drop_view(&view.name), ()).await?;
    conn.execute(ddl::create_view(spec, view), ()).await?;
    record_view(conn, &spec.name, view).await
}

/// Drop a view (and its record) on `conn` if it exists.
async fn drop_view_on(conn: &Connection, name: &str) -> Result<(), BedrockError> {
    conn.execute(ddl::drop_view(name), ()).await?;
    conn.execute(
        ddl::delete_view(),
        vec![turso::Value::Text(name.to_owned())],
    )
    .await?;
    Ok(())
}
