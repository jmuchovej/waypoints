//! Field-level updates: `$set` assignments and `$unset` (set-to-NULL) columns,
//! the mutation counterpart to the [`Filter`](crate::Filter) query DSL. Built as
//! a typed value or parsed from a MongoDB-style update document.

use serde::{Deserialize, Serialize};

use crate::query::{QueryError, ScalarValue, scalar_from_json};

/// A field-level update over a collection's columns.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Update {
    /// Columns to assign, in order.
    pub set: Vec<Assignment>,
    /// Columns to clear to NULL.
    pub unset: Vec<String>,
}

/// One `$set` assignment: a column and the value to give it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub column: String,
    pub value: ScalarValue,
}

impl Update {
    pub fn new() -> Self {
        Self::default()
    }

    /// Assign `column = value`.
    pub fn set(mut self, column: impl Into<String>, value: impl Into<ScalarValue>) -> Self {
        self.set.push(Assignment {
            column: column.into(),
            value: value.into(),
        });
        self
    }

    /// Clear `column` to NULL (only valid for a nullable column).
    pub fn unset(mut self, column: impl Into<String>) -> Self {
        self.unset.push(column.into());
        self
    }

    /// Whether this update would change nothing.
    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.unset.is_empty()
    }

    /// Parse a MongoDB-style update document: `{ "$set": { col: val, … },
    /// "$unset": { col: "" , … } }`. `$unset` values are ignored (only the keys
    /// matter), matching MongoDB.
    pub fn from_json(value: &serde_json::Value) -> Result<Update, QueryError> {
        let object = value.as_object().ok_or(QueryError::NotAnObject)?;
        let mut update = Update::new();
        for (op, operand) in object {
            match op.as_str() {
                "$set" => {
                    let fields = operand.as_object().ok_or(QueryError::BadOperator {
                        op: "$set".into(),
                        expected: "an object of column/value pairs",
                    })?;
                    for (column, field) in fields {
                        update.set.push(Assignment {
                            column: column.clone(),
                            value: scalar_from_json(field)?,
                        });
                    }
                }
                "$unset" => {
                    let fields = operand.as_object().ok_or(QueryError::BadOperator {
                        op: "$unset".into(),
                        expected: "an object of columns",
                    })?;
                    update.unset.extend(fields.keys().cloned());
                }
                other => {
                    return Err(QueryError::UnknownOperator {
                        op: other.to_owned(),
                    });
                }
            }
        }
        Ok(update)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_parses_equivalently() {
        let built = Update::new().set("status", "done").unset("note");
        let parsed = Update::from_json(&serde_json::json!({
            "$set": { "status": "done" },
            "$unset": { "note": "" },
        }))
        .unwrap();
        assert_eq!(built, parsed);
    }

    #[test]
    fn rejects_unknown_operator() {
        let err = Update::from_json(&serde_json::json!({ "$inc": { "n": 1 } })).unwrap_err();
        assert!(matches!(err, QueryError::UnknownOperator { .. }));
    }
}
