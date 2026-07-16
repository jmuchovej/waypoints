use bedrock_core::{CompareOp, Filter, Query, ScalarValue, Update};

/// Quote an identifier for SQL, escaping embedded quotes.
fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn scalar_to_turso(value: &ScalarValue) -> turso::Value {
    match value {
        ScalarValue::Null => turso::Value::Null,
        ScalarValue::Bool(b) => turso::Value::Integer(i64::from(*b)),
        ScalarValue::Int(n) => turso::Value::Integer(*n),
        ScalarValue::Real(n) => turso::Value::Real(*n),
        ScalarValue::Text(s) => turso::Value::Text(s.clone()),
        ScalarValue::Blob(b) => turso::Value::Blob(b.clone()),
    }
}

fn filter_sql(filter: &Filter, params: &mut Vec<turso::Value>) -> String {
    match filter {
        Filter::And(filters) if filters.is_empty() => "1".into(),
        Filter::Or(filters) if filters.is_empty() => "0".into(),
        Filter::And(filters) => join(" AND ", filters, params),
        Filter::Or(filters) => join(" OR ", filters, params),
        Filter::Not(inner) => format!("NOT ({})", filter_sql(inner, params)),
        Filter::Compare { column, op, value } => {
            // A parameterized `= NULL` never matches in SQL; use `IS [NOT] NULL`,
            // and treat an ordered comparison against NULL as unsatisfiable.
            if matches!(value, ScalarValue::Null) {
                return match op {
                    CompareOp::Eq => format!("{} IS NULL", quote(column)),
                    CompareOp::Ne => format!("{} IS NOT NULL", quote(column)),
                    _ => "0".into(),
                };
            }
            params.push(scalar_to_turso(value));
            // Mongo's `$ne` also matches missing/null fields, which a bare SQL
            // `<>` would drop; widen it to include NULL rows.
            if matches!(op, CompareOp::Ne) {
                format!("({col} <> ? OR {col} IS NULL)", col = quote(column))
            } else {
                format!("{} {} ?", quote(column), op.sql())
            }
        }
        Filter::In {
            column,
            values,
            negated,
        } => {
            if values.is_empty() {
                // `x IN ()` is empty: false for IN, true for NOT IN.
                return if *negated { "1".into() } else { "0".into() };
            }
            let placeholders = values
                .iter()
                .map(|value| {
                    params.push(scalar_to_turso(value));
                    "?"
                })
                .collect::<Vec<_>>()
                .join(", ");
            if *negated {
                // Mongo's `$nin` matches missing/null fields too.
                format!(
                    "({col} NOT IN ({p}) OR {col} IS NULL)",
                    col = quote(column),
                    p = placeholders
                )
            } else {
                format!("{} IN ({})", quote(column), placeholders)
            }
        }
        Filter::Exists { column, present } => {
            if *present {
                format!("{} IS NOT NULL", quote(column))
            } else {
                format!("{} IS NULL", quote(column))
            }
        }
        Filter::JsonCompare {
            document,
            path,
            op,
            value,
        } => {
            // json_extract("<doc>", '$.<path>'). The path is a single-quoted SQL
            // string literal, so double any embedded single quotes.
            let extract = format!(
                "json_extract({}, '$.{}')",
                quote(document),
                path.replace('\'', "''")
            );
            if matches!(value, ScalarValue::Null) {
                return match op {
                    CompareOp::Eq => format!("{extract} IS NULL"),
                    CompareOp::Ne => format!("{extract} IS NOT NULL"),
                    _ => "0".into(),
                };
            }
            params.push(scalar_to_turso(value));
            if matches!(op, CompareOp::Ne) {
                format!("({extract} <> ? OR {extract} IS NULL)")
            } else {
                format!("{} {} ?", extract, op.sql())
            }
        }
    }
}

fn join(separator: &str, filters: &[Filter], params: &mut Vec<turso::Value>) -> String {
    filters
        .iter()
        .map(|filter| format!("({})", filter_sql(filter, params)))
        .collect::<Vec<_>>()
        .join(separator)
}

/// Build the `WHERE … ORDER BY … LIMIT …` suffix (with its bound parameters) for
/// a query. The suffix is appended to a `SELECT … FROM …` head by the caller.
pub(crate) fn suffix(query: &Query) -> (String, Vec<turso::Value>) {
    let mut params = Vec::new();
    let mut sql = String::new();

    if let Some(filter) = &query.filter {
        sql.push_str(" WHERE ");
        sql.push_str(&filter_sql(filter, &mut params));
    }
    if !query.sort.is_empty() {
        sql.push_str(" ORDER BY ");
        let keys = query
            .sort
            .iter()
            .map(|s| {
                let direction = if s.descending { "DESC" } else { "ASC" };
                format!("{} {}", quote(&s.column), direction)
            })
            .collect::<Vec<_>>();
        sql.push_str(&keys.join(", "));
    }
    if let Some(limit) = query.limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }

    (sql, params)
}

/// Build the `UPDATE … SET … WHERE …` statement (with its bound parameters) for a
/// field-level update over `table`, restricted to rows matching `filter`. The
/// `SET` parameters precede the `WHERE` parameters. Column existence and
/// nullability are validated by the caller against the collection spec.
pub(crate) fn update_statement(
    table: &str,
    filter: &Filter,
    update: &Update,
) -> (String, Vec<turso::Value>) {
    let mut params = Vec::new();
    let mut assignments = Vec::new();
    for assignment in &update.set {
        assignments.push(format!("{} = ?", quote(&assignment.column)));
        params.push(scalar_to_turso(&assignment.value));
    }
    for column in &update.unset {
        assignments.push(format!("{} = NULL", quote(column)));
    }

    let mut sql = format!(
        "UPDATE {} SET {} WHERE ",
        quote(table),
        assignments.join(", ")
    );
    sql.push_str(&filter_sql(filter, &mut params));
    (sql, params)
}
