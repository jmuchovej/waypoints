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

/// A positional-parameter `INSERT`, columns in virtual-document order.
pub(crate) fn insert(spec: &CollectionSpec) -> String {
    let names: Vec<String> = spec.fields.iter().map(|f| quote(&f.name)).collect();
    let placeholders = vec!["?"; spec.fields.len()].join(", ");
    format!(
        "INSERT INTO {} ({}) VALUES ({})",
        quote(&spec.name),
        names.join(", "),
        placeholders
    )
}

/// `SELECT` of every column in virtual-document order, so result rows align with
/// [`Collection::from_row`](bedrock_core::Collection::from_row).
pub(crate) fn select_all(spec: &CollectionSpec) -> String {
    let names: Vec<String> = spec.fields.iter().map(|f| quote(&f.name)).collect();
    format!("SELECT {} FROM {}", names.join(", "), quote(&spec.name))
}
