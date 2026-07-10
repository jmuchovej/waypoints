//! Every test here runs `EXPLAIN QUERY PLAN` (which makes turso parse and plan
//! the lowered SQL) AND executes the same operation for real, checking the rows.
//! So each case proves three things at once: the SQL is valid turso SQL, it
//! plans the way we expect (index vs. scan), and it returns the right result.
//! Nothing asserts a SQL string in isolation.

use bedrock::{Database, QueryPlan};
use bedrock_core::{ColumnType, Filter, Query, StoredValue, Update, ViewColumn, ViewSpec};
use bedrock_derive::{Collection, Document, Scalar};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct RecId(Uuid);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Document)]
struct Meta {
    note: String,
    weight: i64,
}

#[derive(Debug, Clone, PartialEq, Collection)]
struct Rec {
    #[bedrock(id)]
    id: RecId,
    #[bedrock(index)]
    kind: String,
    rank: i64,
    tag: Option<String>,
    meta: Meta,
}

fn rec(n: u128, kind: &str, rank: i64, tag: Option<&str>) -> Rec {
    Rec {
        id: RecId(Uuid::from_u128(n)),
        kind: kind.into(),
        rank,
        tag: tag.map(Into::into),
        meta: Meta {
            note: "n".into(),
            weight: rank * 10,
        },
    }
}

async fn seeded() -> Database {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Rec>().await.expect("ensure");
    db.insert_all(&[
        rec(1, "a", 10, Some("x")),
        rec(2, "a", 30, Some("y")),
        rec(3, "b", 20, None),
    ])
    .await
    .expect("insert");
    db
}

fn ranks(recs: &[Rec]) -> Vec<i64> {
    recs.iter().map(|r| r.rank).collect()
}

#[tokio::test]
async fn indexed_equality_seeks_the_index() {
    let db = seeded().await;
    let query = Query::new()
        .filter(Filter::eq("kind", "a"))
        .sort_asc("rank");

    let plan = db.explain_query::<Rec>(&query).await.expect("explain");
    // The plan seeks the `kind` index rather than scanning the table.
    assert!(
        plan.uses_index(),
        "expected an index seek, got {:?}",
        plan.steps
    );
    assert!(plan.steps.iter().any(|s| s.contains("kind")));

    // ...and the same lowering returns the right rows when executed.
    let rows = db.query::<Rec>(&query).await.expect("query");
    assert_eq!(ranks(&rows), vec![10, 30]);
}

#[tokio::test]
async fn non_indexed_filter_scans() {
    let db = seeded().await;
    let query = Query::new().filter(Filter::gt("rank", 15i64));

    let plan = db.explain_query::<Rec>(&query).await.expect("explain");
    assert!(plan.scans(), "expected a scan, got {:?}", plan.steps);
    assert!(!plan.uses_index(), "rank is unindexed: {:?}", plan.steps);

    let rows = db.query::<Rec>(&query).await.expect("query");
    assert_eq!(
        ranks(&rows)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        [20, 30].into()
    );
}

#[tokio::test]
async fn ne_lowering_is_valid_and_null_inclusive() {
    let db = seeded().await;
    let query = Query::new().filter(Filter::ne("tag", "x"));

    // The plan proves the widened `(tag <> ? OR tag IS NULL)` SQL parses/plans,
    // and carries the bound parameter.
    let plan = db.explain_query::<Rec>(&query).await.expect("explain");
    assert_eq!(plan.params, vec![StoredValue::Text("x".into())]);
    assert!(
        plan.sql.contains("IS NULL"),
        "ne must widen to include NULL: {}",
        plan.sql
    );

    // Executed: matches "y" AND the NULL-tag row — Mongo semantics.
    let rows = db.query::<Rec>(&query).await.expect("query");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|r| r.tag.is_none()));
}

#[tokio::test]
async fn nin_lowering_is_valid_and_null_inclusive() {
    let db = seeded().await;
    let query = Query::new().filter(Filter::not_in("tag", ["x".into(), "y".into()]));

    let plan = db.explain_query::<Rec>(&query).await.expect("explain");
    assert_eq!(
        plan.params,
        vec![StoredValue::Text("x".into()), StoredValue::Text("y".into())]
    );
    assert!(plan.sql.contains("NOT IN"));
    assert!(plan.sql.contains("IS NULL"));

    let rows = db.query::<Rec>(&query).await.expect("query");
    // Only the NULL-tag row survives $nin ["x","y"].
    assert_eq!(rows.len(), 1);
    assert!(rows[0].tag.is_none());
}

#[tokio::test]
async fn explain_update_plans_and_matches_execution() {
    let db = seeded().await;
    let filter = Filter::eq("kind", "a");
    let update = Update::new().set("rank", 99i64);

    // EXPLAIN QUERY PLAN over the UPDATE proves the $set SQL is valid turso.
    let plan = db
        .explain_update::<Rec>(&filter, &update)
        .await
        .expect("explain");
    assert!(plan.sql.starts_with("UPDATE"), "{}", plan.sql);
    // SET binds precede WHERE binds: [99, "a"].
    assert_eq!(
        plan.params,
        vec![StoredValue::Integer(99), StoredValue::Text("a".into())]
    );
    assert!(plan.uses_index(), "the WHERE seeks kind: {:?}", plan.steps);

    // Running the real update affects exactly the two "a" rows.
    let affected = db
        .update_where::<Rec>(&filter, &update)
        .await
        .expect("update");
    assert_eq!(affected, 2);
    let bumped = db
        .query::<Rec>(&Query::new().filter(Filter::eq("rank", 99i64)))
        .await
        .expect("query");
    assert_eq!(bumped.len(), 2);
}

#[tokio::test]
async fn explain_view_query_plans_and_matches_execution() {
    let db = seeded().await;
    let view = ViewSpec {
        name: "rec_weight".into(),
        scope: None,
        columns: vec![ViewColumn {
            name: "weight".into(),
            document: "meta".into(),
            path: "weight".into(),
            column_type: ColumnType::Integer,
        }],
    };
    db.declare_view::<Rec>(&view).await.expect("declare");

    let query = Query::new()
        .filter(Filter::gte("weight", 200i64))
        .sort_desc("weight");
    let plan = db
        .explain_view_query("rec_weight", &query)
        .await
        .expect("explain");
    // The view query plans (proving the projected-column SQL is valid).
    assert!(!plan.steps.is_empty(), "expected a plan for the view query");
    assert!(plan.sql.contains("rec_weight"));

    let rows = db
        .query_view("rec_weight", &query)
        .await
        .expect("query view");
    let weights: Vec<i64> = rows
        .iter()
        .map(|r| match r[1] {
            StoredValue::Integer(n) => n,
            _ => -1,
        })
        .collect();
    assert_eq!(weights, vec![300, 200]);
}

#[tokio::test]
async fn query_plan_helpers_are_consistent() {
    // A tiny guard that uses_index/scans reflect the steps, on a real plan.
    let db = seeded().await;
    let indexed: QueryPlan = db
        .explain_query::<Rec>(&Query::new().filter(Filter::eq("kind", "b")))
        .await
        .expect("explain");
    assert!(indexed.uses_index() && !indexed.scans());
}
