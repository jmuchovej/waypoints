use serde::{Deserialize, Serialize};

/// The logical type of a promotable scalar column.
///
/// This is the *logical* type bedrock reasons about — the mapping to concrete
/// turso/SQLite affinities and DDL lives in the engine layer, not here. Keeping
/// it logical means the schema vocabulary stays free of engine quirks (STRICT
/// affinities, experimental custom types) that belong at the boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    Integer,
    Real,
    Text,
    Blob,
    Boolean,
    Uuid,
    Date,
    Time,
    Timestamp,
}

/// A value in its storage wire form — the representation that crosses the
/// boundary between Rust and the engine.
///
/// Scalars land in one of the SQLite storage classes; document fields (structs,
/// enums, maps, sequences) land in [`Json`](StoredValue::Json), which the engine
/// layer binds as `jsonb`.
#[derive(Clone, Debug, PartialEq)]
pub enum StoredValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Json(serde_json::Value),
}

impl StoredValue {
    /// A stable, human-readable name for the variant, used in error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            StoredValue::Null => "null",
            StoredValue::Integer(_) => "integer",
            StoredValue::Real(_) => "real",
            StoredValue::Text(_) => "text",
            StoredValue::Blob(_) => "blob",
            StoredValue::Json(_) => "json",
        }
    }
}
