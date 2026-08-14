//! Query-plan diagnostics: the SQL a bedrock operation lowers to, together with
//! the engine's `EXPLAIN QUERY PLAN` for it.
//!
//! The point is verification, not production telemetry: a [`QueryPlan`] is only
//! ever produced by running the lowered SQL through turso's planner, so the
//! `sql` it carries is provably valid turso SQL — never a hand-asserted string.
//! Use it in tests to pin down how the Mongo-style operators translate and to
//! catch a filter that should seek an index but plans as a scan.

use crate::{BedrockError, core::StoredValue, value};
use turso::Connection;

/// A lowered bedrock operation: the SQL, its bound parameters, and the engine's
/// `EXPLAIN QUERY PLAN` steps (one `detail` string per plan row).
#[derive(Debug, Clone, PartialEq)]
pub struct QueryPlan {
    /// The SQL that the operation executes.
    pub sql: String,
    /// The parameters bound into `sql`, in order.
    pub params: Vec<StoredValue>,
    /// The `EXPLAIN QUERY PLAN` detail lines, e.g. `"SEARCH t USING INDEX …"`
    /// or `"SCAN t"`.
    pub steps: Vec<String>,
}

impl QueryPlan {
    /// Whether any step seeks an index (`… USING INDEX …`, including a covering
    /// index). A convenience for the common "did this filter hit its index?"
    /// assertion; inspect [`steps`](Self::steps) directly for finer checks.
    pub fn uses_index(&self) -> bool {
        self.steps.iter().any(|step| step.contains("USING INDEX"))
    }

    /// Whether any step is a full table scan (`SCAN …`).
    pub fn scans(&self) -> bool {
        self.steps
            .iter()
            .any(|step| step.trim_start().starts_with("SCAN"))
    }
}

/// Build a [`QueryPlan`] for `sql`/`params` by running `EXPLAIN QUERY PLAN`
/// against `conn`. The EXPLAIN itself parses and plans the SQL, so a returned
/// plan is proof the SQL is valid.
pub(crate) async fn plan(
    conn: &Connection,
    sql: String,
    params: Vec<turso::Value>,
) -> Result<QueryPlan, BedrockError> {
    let mut rows = conn
        .query(format!("EXPLAIN QUERY PLAN {sql}"), params.clone())
        .await?;
    let mut steps = Vec::new();
    while let Some(row) = rows.next().await? {
        // EXPLAIN QUERY PLAN rows are (id, parent, _, detail); the detail is the
        // last column.
        let last = row.column_count().saturating_sub(1);
        if let turso::Value::Text(detail) = row.get_value(last)? {
            steps.push(detail);
        }
    }
    drop(rows);

    let params = params
        .into_iter()
        .map(value::from_turso_scalar)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(QueryPlan { sql, params, steps })
}
