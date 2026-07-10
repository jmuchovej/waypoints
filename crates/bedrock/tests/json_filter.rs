//! `Filter::json_eq` reaches a discriminant INSIDE a jsonb document column,
//! filtered on the base table (no view), alongside ordinary column filters —
//! the shape `store::query::mark::list` uses to query marks by kind.

use bedrock::{Collection, Database, Document, Filter, Query, Scalar};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct RecId(Uuid);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Document)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Body {
    Note { title: String },
    Task { title: String },
}

#[derive(Debug, Clone, PartialEq, Collection)]
struct Rec {
    #[bedrock(id)]
    id: RecId,
    data: Body,
    deleted: Option<String>,
}

fn rec(n: u128, data: Body, deleted: Option<&str>) -> Rec {
    Rec {
        id: RecId(Uuid::from_u128(n)),
        data,
        deleted: deleted.map(Into::into),
    }
}

#[tokio::test]
async fn json_eq_filters_a_document_discriminant_with_a_column_filter() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Rec>().await.expect("ensure");
    db.insert_all(vec![
        rec(1, Body::Note { title: "a".into() }, None),
        rec(2, Body::Task { title: "b".into() }, None),
        rec(3, Body::Note { title: "c".into() }, Some("x")), // a "deleted" note
    ])
    .await
    .expect("insert");

    // Notes that aren't deleted: json_eq on the tag inside `data` + IS NULL on a column.
    let q = Query::new().filter(Filter::all([
        Filter::json_eq("data", "kind", "note"),
        Filter::missing("deleted"),
    ]));
    let got = db.query::<Rec>(&q).await.expect("query");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, RecId(Uuid::from_u128(1)));

    // Tasks: the other discriminant value.
    let tasks = db
        .query::<Rec>(&Query::new().filter(Filter::json_eq("data", "kind", "task")))
        .await
        .expect("query");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, RecId(Uuid::from_u128(2)));
}
