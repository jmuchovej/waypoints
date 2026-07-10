use bedrock_core::StoredValue;

/// Convert a stored value into a turso parameter value.
///
/// Documents ride as compact JSON text; every other variant maps one-to-one
/// onto a SQLite storage class.
pub(crate) fn to_turso(value: StoredValue) -> Result<turso::Value, serde_json::Error> {
    Ok(match value {
        StoredValue::Null => turso::Value::Null,
        StoredValue::Integer(n) => turso::Value::Integer(n),
        StoredValue::Real(n) => turso::Value::Real(n),
        StoredValue::Text(s) => turso::Value::Text(s),
        StoredValue::Blob(b) => turso::Value::Blob(b),
        StoredValue::Json(v) => turso::Value::Text(serde_json::to_string(&v)?),
    })
}

/// Convert a turso column value back into a stored value.
///
/// The caller passes `is_document` because a document column is physically TEXT
/// (JSON) but must reassemble as [`StoredValue::Json`] for the field's decoder;
/// the column's spec is the only thing that distinguishes it from a plain text
/// scalar.
pub(crate) fn from_turso(
    value: turso::Value,
    is_document: bool,
) -> Result<StoredValue, serde_json::Error> {
    Ok(match value {
        turso::Value::Null => StoredValue::Null,
        turso::Value::Integer(n) => StoredValue::Integer(n),
        turso::Value::Real(n) => StoredValue::Real(n),
        turso::Value::Blob(b) => StoredValue::Blob(b),
        turso::Value::Text(s) if is_document => StoredValue::Json(serde_json::from_str(&s)?),
        turso::Value::Text(s) => StoredValue::Text(s),
    })
}
