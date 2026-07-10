//! The query DSL: a typed [`Filter`] tree (plus sort and limit) that the engine
//! turns into SQL, with a MongoDB-style JSON document as an alternate front end.
//!
//! Columns are flat names — a base scalar column, or a materialized view's
//! projected column. Nested document paths are addressed by projecting them into
//! a view first (they become view columns); the filter then names that column.

use serde::{Deserialize, Serialize};

/// A predicate over a collection's (or view's) columns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Filter {
    /// All sub-filters hold.
    And(Vec<Filter>),
    /// Any sub-filter holds.
    Or(Vec<Filter>),
    /// The sub-filter does not hold.
    Not(Box<Filter>),
    /// A column compares to a value.
    Compare {
        column: String,
        op: CompareOp,
        value: ScalarValue,
    },
    /// A column is (`negated = false`) or is not (`true`) among a set of values.
    In {
        column: String,
        values: Vec<ScalarValue>,
        negated: bool,
    },
    /// A column is non-null (`present = true`) or null (`false`).
    Exists { column: String, present: bool },
}

/// A scalar comparison operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

/// A scalar bound in a filter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScalarValue {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    Text(String),
}

/// A sort key: a column and a direction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sort {
    pub column: String,
    pub descending: bool,
}

/// A whole query: an optional filter, ordered sort keys, and an optional limit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub filter: Option<Filter>,
    pub sort: Vec<Sort>,
    pub limit: Option<u64>,
}

impl CompareOp {
    /// The SQL operator token.
    pub fn sql(self) -> &'static str {
        match self {
            CompareOp::Eq => "=",
            CompareOp::Ne => "<>",
            CompareOp::Gt => ">",
            CompareOp::Gte => ">=",
            CompareOp::Lt => "<",
            CompareOp::Lte => "<=",
        }
    }
}

macro_rules! compare_builders {
    ($($name:ident => $op:ident),* $(,)?) => {$(
        #[doc = concat!("A `", stringify!($op), "` comparison on a column.")]
        pub fn $name(column: impl Into<String>, value: impl Into<ScalarValue>) -> Self {
            Filter::Compare { column: column.into(), op: CompareOp::$op, value: value.into() }
        }
    )*};
}

impl Filter {
    compare_builders! {
        eq => Eq, ne => Ne, gt => Gt, gte => Gte, lt => Lt, lte => Lte,
    }

    /// All of `filters` must hold.
    pub fn all(filters: impl IntoIterator<Item = Filter>) -> Self {
        Filter::And(filters.into_iter().collect())
    }

    /// Any of `filters` must hold.
    pub fn any(filters: impl IntoIterator<Item = Filter>) -> Self {
        Filter::Or(filters.into_iter().collect())
    }

    /// Negate a filter.
    #[allow(clippy::should_implement_trait)] // a `Filter` constructor, not `Not::not`
    pub fn not(filter: Filter) -> Self {
        Filter::Not(Box::new(filter))
    }

    /// A column is among `values`.
    pub fn is_in(column: impl Into<String>, values: impl IntoIterator<Item = ScalarValue>) -> Self {
        Filter::In {
            column: column.into(),
            values: values.into_iter().collect(),
            negated: false,
        }
    }

    /// A column is not among `values`.
    pub fn not_in(
        column: impl Into<String>,
        values: impl IntoIterator<Item = ScalarValue>,
    ) -> Self {
        Filter::In {
            column: column.into(),
            values: values.into_iter().collect(),
            negated: true,
        }
    }

    /// A column is non-null.
    pub fn exists(column: impl Into<String>) -> Self {
        Filter::Exists {
            column: column.into(),
            present: true,
        }
    }

    /// A column is null.
    pub fn missing(column: impl Into<String>) -> Self {
        Filter::Exists {
            column: column.into(),
            present: false,
        }
    }

    /// Parse a MongoDB-style filter document into a [`Filter`].
    ///
    /// `{ "a": 1, "b": { "$gt": 2 } }` is an implicit AND of field predicates;
    /// `$and`/`$or`/`$nor` combine; field values are either a scalar (equality)
    /// or an object of `$eq/$ne/$gt/$gte/$lt/$lte/$in/$nin/$exists` operators.
    pub fn from_json(value: &serde_json::Value) -> Result<Filter, QueryError> {
        let object = value.as_object().ok_or(QueryError::NotAnObject)?;
        let mut clauses = Vec::new();
        for (key, sub) in object {
            clauses.push(match key.as_str() {
                "$and" => Filter::And(parse_branches(sub)?),
                "$or" => Filter::Or(parse_branches(sub)?),
                "$nor" => Filter::Not(Box::new(Filter::Or(parse_branches(sub)?))),
                other if other.starts_with('$') => {
                    return Err(QueryError::UnknownOperator {
                        op: other.to_owned(),
                    });
                }
                column => parse_field(column, sub)?,
            });
        }
        Ok(match clauses.len() {
            1 => clauses.pop().expect("length checked"),
            _ => Filter::And(clauses),
        })
    }
}

fn parse_branches(value: &serde_json::Value) -> Result<Vec<Filter>, QueryError> {
    value
        .as_array()
        .ok_or(QueryError::BadOperator {
            op: "$and/$or/$nor".into(),
            expected: "an array of filters",
        })?
        .iter()
        .map(Filter::from_json)
        .collect()
}

fn parse_field(column: &str, value: &serde_json::Value) -> Result<Filter, QueryError> {
    let Some(object) = value.as_object() else {
        // A bare scalar is shorthand for equality.
        return Ok(Filter::eq(column, scalar_from_json(value)?));
    };
    if !object.keys().any(|k| k.starts_with('$')) {
        // A non-operator object as a value is not something we can compare to a column.
        return Err(QueryError::UnsupportedValue {
            value: value.to_string(),
        });
    }

    let mut clauses = Vec::new();
    for (op, operand) in object {
        clauses.push(match op.as_str() {
            "$eq" => Filter::eq(column, scalar_from_json(operand)?),
            "$ne" => Filter::ne(column, scalar_from_json(operand)?),
            "$gt" => Filter::gt(column, scalar_from_json(operand)?),
            "$gte" => Filter::gte(column, scalar_from_json(operand)?),
            "$lt" => Filter::lt(column, scalar_from_json(operand)?),
            "$lte" => Filter::lte(column, scalar_from_json(operand)?),
            "$in" => Filter::is_in(column, scalar_array(operand)?),
            "$nin" => Filter::not_in(column, scalar_array(operand)?),
            "$exists" => {
                let present = operand.as_bool().ok_or(QueryError::BadOperator {
                    op: "$exists".into(),
                    expected: "a boolean",
                })?;
                Filter::Exists {
                    column: column.to_owned(),
                    present,
                }
            }
            other => {
                return Err(QueryError::UnknownOperator {
                    op: other.to_owned(),
                });
            }
        });
    }
    Ok(match clauses.len() {
        1 => clauses.pop().expect("length checked"),
        _ => Filter::And(clauses),
    })
}

fn scalar_array(value: &serde_json::Value) -> Result<Vec<ScalarValue>, QueryError> {
    value
        .as_array()
        .ok_or(QueryError::BadOperator {
            op: "$in/$nin".into(),
            expected: "an array",
        })?
        .iter()
        .map(scalar_from_json)
        .collect()
}

pub(crate) fn scalar_from_json(value: &serde_json::Value) -> Result<ScalarValue, QueryError> {
    use serde_json::Value;
    Ok(match value {
        Value::Null => ScalarValue::Null,
        Value::Bool(b) => ScalarValue::Bool(*b),
        Value::Number(n) if n.is_i64() => ScalarValue::Int(n.as_i64().expect("is_i64")),
        Value::Number(n) if n.is_f64() => ScalarValue::Real(n.as_f64().expect("is_f64")),
        // A number that fits neither i64 nor f64 exactly — a u64 above i64::MAX,
        // or (with `arbitrary_precision`) a high-precision decimal. No bedrock
        // column can hold it — surface it rather than bind a lossy float.
        Value::Number(_) => {
            return Err(QueryError::UnsupportedValue {
                value: value.to_string(),
            });
        }
        Value::String(s) => ScalarValue::Text(s.clone()),
        Value::Array(_) | Value::Object(_) => {
            return Err(QueryError::UnsupportedValue {
                value: value.to_string(),
            });
        }
    })
}

impl Query {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn sort_asc(mut self, column: impl Into<String>) -> Self {
        self.sort.push(Sort {
            column: column.into(),
            descending: false,
        });
        self
    }

    pub fn sort_desc(mut self, column: impl Into<String>) -> Self {
        self.sort.push(Sort {
            column: column.into(),
            descending: true,
        });
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl From<bool> for ScalarValue {
    fn from(value: bool) -> Self {
        ScalarValue::Bool(value)
    }
}
impl From<i64> for ScalarValue {
    fn from(value: i64) -> Self {
        ScalarValue::Int(value)
    }
}
impl From<i32> for ScalarValue {
    fn from(value: i32) -> Self {
        ScalarValue::Int(value.into())
    }
}
impl From<f64> for ScalarValue {
    fn from(value: f64) -> Self {
        ScalarValue::Real(value)
    }
}
impl From<&str> for ScalarValue {
    fn from(value: &str) -> Self {
        ScalarValue::Text(value.to_owned())
    }
}
impl From<String> for ScalarValue {
    fn from(value: String) -> Self {
        ScalarValue::Text(value)
    }
}

/// A malformed MongoDB-style filter document.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum QueryError {
    #[error("a filter must be a JSON object")]
    NotAnObject,

    #[error("operator {op:?} expects {expected}")]
    BadOperator { op: String, expected: &'static str },

    #[error("unknown operator {op:?}")]
    UnknownOperator { op: String },

    #[error("{value} is not a supported scalar value")]
    UnsupportedValue { value: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_implicit_and_of_fields() {
        let filter = Filter::from_json(&serde_json::json!({
            "status": "published",
            "rank": { "$gt": 5 },
        }))
        .unwrap();
        // order of object keys is preserved (serde_json preserve_order)
        assert_eq!(
            filter,
            Filter::And(vec![
                Filter::eq("status", "published"),
                Filter::gt("rank", 5i64),
            ])
        );
    }

    #[test]
    fn parses_operators_and_logical() {
        let filter = Filter::from_json(&serde_json::json!({
            "$or": [
                { "kind": { "$in": ["a", "b"] } },
                { "score": { "$exists": true } },
            ]
        }))
        .unwrap();
        assert_eq!(
            filter,
            Filter::Or(vec![
                Filter::is_in(
                    "kind",
                    [ScalarValue::Text("a".into()), ScalarValue::Text("b".into())]
                ),
                Filter::exists("score"),
            ])
        );
    }

    #[test]
    fn rejects_unknown_operator() {
        let err = Filter::from_json(&serde_json::json!({ "x": { "$regex": "a" } })).unwrap_err();
        assert!(matches!(err, QueryError::UnknownOperator { .. }));
    }

    #[test]
    fn rejects_out_of_range_integer() {
        // 10^19 fits u64 but not i64 — must error, not silently become a lossy float.
        let err =
            Filter::from_json(&serde_json::json!({ "count": 10_000_000_000_000_000_000_u64 }))
                .unwrap_err();
        assert!(matches!(err, QueryError::UnsupportedValue { .. }));
    }
}
