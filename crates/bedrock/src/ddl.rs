use bedrock_core::{CollectionSpec, ColumnType, FieldKind};

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
