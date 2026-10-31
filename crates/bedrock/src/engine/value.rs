use crate::{
    BedrockError,
    core::{CollectionSpec, ColumnType, FieldKind, StoredValue},
};

/// Bind a row of stored values into turso parameters, in `spec.fields` order,
/// rejecting sub-millisecond temporals first.
pub(crate) fn to_params(
    spec: &CollectionSpec,
    row: &[StoredValue],
) -> Result<Vec<turso::Value>, BedrockError> {
    let mut params = Vec::with_capacity(row.len());
    for (field, value) in spec.fields.iter().zip(row) {
        reject_submillis(field.kind, value, &field.name)?;
        params.push(to_turso(value)?);
    }
    Ok(params)
}

/// Convert a single stored value into a turso parameter value.
///
/// Documents ride as compact JSON text (a `jsonb` column ENCODEs it); every
/// other variant maps one-to-one onto a storage class.
pub(crate) fn to_turso(value: &StoredValue) -> Result<turso::Value, serde_json::Error> {
    Ok(match value {
        StoredValue::Null => turso::Value::Null,
        StoredValue::Integer(n) => turso::Value::Integer(*n),
        StoredValue::Real(n) => turso::Value::Real(*n),
        StoredValue::Text(s) => turso::Value::Text(s.clone()),
        StoredValue::Blob(b) => turso::Value::Blob(b.clone()),
        StoredValue::Json(v) => turso::Value::Text(serde_json::to_string(v)?),
    })
}

/// Read a full result row into stored values, using the spec's per-column
/// document flags (a `jsonb` column is physically text but reassembles as
/// [`StoredValue::Json`]).
pub(crate) fn read_row(
    row: &turso::Row,
    is_document: &[bool],
) -> Result<Vec<StoredValue>, BedrockError> {
    let mut stored = Vec::with_capacity(is_document.len());
    for (index, &is_doc) in is_document.iter().enumerate() {
        stored.push(from_turso(row.get_value(index)?, is_doc)?);
    }
    Ok(stored)
}

/// Read a scalar (non-document) turso value into a stored value — used for
/// materialized-view rows, whose columns are all `json_extract` scalars.
pub(crate) fn from_turso_scalar(value: turso::Value) -> Result<StoredValue, serde_json::Error> {
    from_turso(value, false)
}

/// Reject a sub-millisecond temporal value before binding.
///
/// The engine's `time`/`timestamp` columns truncate to milliseconds, so finer
/// precision would be silently lost. Encoded temporals are `HH:MM:SS.fff…` with
/// trailing zeros trimmed, so a fractional part longer than three digits is
/// sub-millisecond.
fn reject_submillis(kind: FieldKind, value: &StoredValue, field: &str) -> Result<(), BedrockError> {
    let temporal = matches!(
        kind,
        FieldKind::Column(ColumnType::Time | ColumnType::Timestamp)
    );
    if temporal
        && let StoredValue::Text(text) = value
        && let Some(dot) = text.find('.')
        && text[dot + 1..].len() > 3
    {
        return Err(BedrockError::SubMillisecond {
            field: field.to_owned(),
        });
    }
    Ok(())
}

fn from_turso(value: turso::Value, is_document: bool) -> Result<StoredValue, serde_json::Error> {
    Ok(match value {
        turso::Value::Null => StoredValue::Null,
        turso::Value::Integer(n) => StoredValue::Integer(n),
        turso::Value::Real(n) => StoredValue::Real(n),
        turso::Value::Blob(b) => StoredValue::Blob(b),
        turso::Value::Text(s) if is_document => StoredValue::Json(serde_json::from_str(&s)?),
        turso::Value::Text(s) => StoredValue::Text(s),
    })
}
