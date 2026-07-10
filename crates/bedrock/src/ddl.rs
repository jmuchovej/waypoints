use bedrock_core::{CollectionSpec, ColumnType, FieldKind, FieldSpec, IndexSpec, ViewSpec};

/// The engine storage type for a field.
///
/// Behind `experimental_custom_types` + STRICT, uuid/boolean/jsonb and the
/// temporal types are real engine types (raw-SQL-inspectable, and `json_extract`
/// reaches into `jsonb`).
fn sql_type(kind: FieldKind) -> &'static str {
    match kind {
        FieldKind::Document => "jsonb",
        FieldKind::Column(column) => match column {
            ColumnType::Integer => "INTEGER",
            ColumnType::Real => "REAL",
            ColumnType::Text => "TEXT",
            ColumnType::Blob => "BLOB",
            ColumnType::Boolean => "boolean",
            ColumnType::Uuid => "uuid",
            ColumnType::Date => "date",
            ColumnType::Time => "time",
            ColumnType::Timestamp => "timestamp",
        },
    }
}

/// Quote an identifier for SQL, escaping embedded quotes.
fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// A comma-separated list of quoted column names, in virtual-document order.
fn column_list(spec: &CollectionSpec) -> String {
    spec.fields
        .iter()
        .map(|f| quote(&f.name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `CREATE TABLE IF NOT EXISTS` for a collection, columns in virtual-document order.
pub(crate) fn create_table(spec: &CollectionSpec) -> String {
    let columns: Vec<String> = spec
        .fields
        .iter()
        .map(|field| {
            let mut column = format!("{} {}", quote(&field.name), sql_type(field.kind));
            if field.name == spec.id.field {
                column.push_str(" PRIMARY KEY");
            }
            if !field.nullable {
                column.push_str(" NOT NULL");
            }
            column
        })
        .collect();
    format!(
        "CREATE TABLE IF NOT EXISTS {} (\n  {}\n) STRICT",
        quote(&spec.name),
        columns.join("\n, ")
    )
}

/// A positional-parameter `INSERT`. When `replace`, it is `INSERT OR REPLACE`
/// (an upsert keyed on the primary key). Params are the full row in
/// virtual-document order.
pub(crate) fn insert(spec: &CollectionSpec, replace: bool) -> String {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    let placeholders = vec!["?"; spec.fields.len()].join(", ");
    format!(
        "{} INTO {} ({}) VALUES ({})",
        verb,
        quote(&spec.name),
        column_list(spec),
        placeholders
    )
}

/// `UPDATE … SET (every column) WHERE id = ?`. Params are the full row followed
/// by the id value again for the `WHERE`.
pub(crate) fn update(spec: &CollectionSpec) -> String {
    let assignments = spec
        .fields
        .iter()
        .map(|f| format!("{} = ?", quote(&f.name)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "UPDATE {} SET {} WHERE {} = ?",
        quote(&spec.name),
        assignments,
        quote(&spec.id.field)
    )
}

/// `SELECT` of every column, in virtual-document order, so result rows align
/// with [`Collection::from_row`](bedrock_core::Collection::from_row).
pub(crate) fn select_all(spec: &CollectionSpec) -> String {
    format!("SELECT {} FROM {}", column_list(spec), quote(&spec.name))
}

/// `SELECT` one row by primary key. Param is the id value.
pub(crate) fn select_by_id(spec: &CollectionSpec) -> String {
    format!(
        "SELECT {} FROM {} WHERE {} = ?",
        column_list(spec),
        quote(&spec.name),
        quote(&spec.id.field)
    )
}

/// `DELETE` one row by primary key. Param is the id value.
pub(crate) fn delete_by_id(spec: &CollectionSpec) -> String {
    format!(
        "DELETE FROM {} WHERE {} = ?",
        quote(&spec.name),
        quote(&spec.id.field)
    )
}

/// `SELECT COUNT(*)` over the collection.
pub(crate) fn count(spec: &CollectionSpec) -> String {
    format!("SELECT COUNT(*) FROM {}", quote(&spec.name))
}

/// `CREATE [UNIQUE] INDEX IF NOT EXISTS` for one declared index.
pub(crate) fn create_index(spec: &CollectionSpec, index: &IndexSpec) -> String {
    let unique = if index.unique { "UNIQUE " } else { "" };
    let columns = index
        .columns
        .iter()
        .map(|c| quote(c))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "CREATE {}INDEX IF NOT EXISTS {} ON {} ({})",
        unique,
        quote(&index.name),
        quote(&spec.name),
        columns
    )
}

/// A dotted path as a JSON path expression: `readings.abc` → `$.readings.abc`.
/// Segments are bare object keys; array indices and wildcards are not supported.
///
/// The result is inlined into a single-quoted SQL string literal, so embedded
/// single quotes are escaped here — the same reason a scope's discriminant value
/// is escaped where it is inlined.
fn json_path(path: &str) -> String {
    let mut out = String::from("$");
    for segment in path.split('.') {
        out.push('.');
        out.push_str(segment);
    }
    out.replace('\'', "''")
}

/// `CREATE MATERIALIZED VIEW` projecting document paths into typed columns.
///
/// The base id is projected first, so a query can narrow on the base table and
/// join to the view. An optional scope inlines a discriminant equality — DDL
/// cannot bind parameters, so the value is escaped and inlined.
pub(crate) fn create_view(spec: &CollectionSpec, view: &ViewSpec) -> String {
    let mut projections = vec![quote(&spec.id.field)];
    for column in &view.columns {
        projections.push(format!(
            "json_extract({}, '{}') AS {}",
            quote(&column.document),
            json_path(&column.path),
            quote(&column.name)
        ));
    }
    let mut sql = format!(
        "CREATE MATERIALIZED VIEW {} AS SELECT {} FROM {}",
        quote(&view.name),
        projections.join(", "),
        quote(&spec.name)
    );
    if let Some(scope) = &view.scope {
        sql.push_str(&format!(
            " WHERE json_extract({}, '{}') = '{}'",
            quote(&scope.document),
            json_path(&scope.path),
            scope.equals.replace('\'', "''")
        ));
    }
    sql
}

/// `DROP VIEW IF EXISTS` (materialized views drop with the plain form).
pub(crate) fn drop_view(name: &str) -> String {
    format!("DROP VIEW IF EXISTS {}", quote(name))
}

/// `SELECT *` over a view.
pub(crate) fn select_view(name: &str) -> String {
    format!("SELECT * FROM {}", quote(name))
}

/// `ALTER TABLE … ADD COLUMN` for a new nullable column (migrations only add
/// nullable columns, so there is no `NOT NULL`).
pub(crate) fn alter_add_column(spec: &CollectionSpec, field: &FieldSpec) -> String {
    format!(
        "ALTER TABLE {} ADD COLUMN {} {}",
        quote(&spec.name),
        quote(&field.name),
        sql_type(field.kind)
    )
}

/// `DROP INDEX IF EXISTS`.
pub(crate) fn drop_index(name: &str) -> String {
    format!("DROP INDEX IF EXISTS {}", quote(name))
}

/// The system table storing each collection's last-applied spec, for migration
/// diffing across opens.
pub(crate) fn create_meta_table() -> &'static str {
    "CREATE TABLE IF NOT EXISTS \"bedrock_collections\" (\"name\" TEXT PRIMARY KEY, \"spec\" TEXT NOT NULL) STRICT"
}

/// Upsert a collection's stored spec. Params: name, spec-json.
pub(crate) fn store_spec() -> &'static str {
    "INSERT OR REPLACE INTO \"bedrock_collections\" (\"name\", \"spec\") VALUES (?, ?)"
}

/// Read a collection's stored spec. Param: name.
pub(crate) fn read_spec() -> &'static str {
    "SELECT \"spec\" FROM \"bedrock_collections\" WHERE \"name\" = ?"
}
