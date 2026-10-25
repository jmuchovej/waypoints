use bedrock::Database;
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
    meta: Meta,
}

fn rec(n: u128, kind: &str, rank: i64, note: &str, weight: i64) -> Rec {
    Rec {
        id: RecId(Uuid::from_u128(n)),
        kind: kind.into(),
        rank,
        meta: Meta {
            note: note.into(),
            weight,
        },
    }
}

async fn seeded() -> Database {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Rec>().await.expect("ensure");
    db.insert_all(vec![
        rec(1, "a", 10, "x", 100),
        rec(2, "a", 30, "y", 200),
        rec(3, "b", 20, "z", 300),
    ])
    .await
    .expect("insert");
    db
}

fn ranks(recs: &[Rec]) -> Vec<i64> {
    recs.iter().map(|r| r.rank).collect()
}

#[tokio::test]
async fn query_base_with_filter_enum() {
    let db = seeded().await;

    // kind = "a" AND rank > 5, sorted rank desc
    let query = Query::new()
        .filter(Filter::all([
            Filter::eq("kind", "a"),
            Filter::gt("rank", 5i64),
        ]))
        .sort_desc("rank");
    assert_eq!(ranks(&db.query::<Rec>(&query).await.unwrap()), vec![30, 10]);

    // $in
    let query = Query::new()
        .filter(Filter::is_in("kind", ["b".into(), "c".into()]))
        .sort_asc("rank");
    assert_eq!(ranks(&db.query::<Rec>(&query).await.unwrap()), vec![20]);

    // limit
    let top = db
        .query::<Rec>(&Query::new().sort_desc("rank").limit(1))
        .await
        .unwrap();
    assert_eq!(ranks(&top), vec![30]);
}

#[tokio::test]
async fn query_base_with_json_filter() {
    let db = seeded().await;

    let filter = Filter::from_json(&serde_json::json!({
        "kind": "a",
        "rank": { "$gte": 30 },
    }))
    .unwrap();
    let got = db.query::<Rec>(&Query::new().filter(filter)).await.unwrap();
    assert_eq!(ranks(&got), vec![30]);
}

#[tokio::test]
async fn query_materialized_view() {
    let db = seeded().await;

    // project the document paths meta.note / meta.weight into typed view columns
    let view = ViewSpec {
        name: "rec_view".into(),
        scope: None,
        columns: vec![
            ViewColumn {
                name: "note".into(),
                document: "meta".into(),
                ns: None,
                path: "note".into(),
                column_type: ColumnType::Text,
            },
            ViewColumn {
                name: "weight".into(),
                document: "meta".into(),
                ns: None,
                path: "weight".into(),
                column_type: ColumnType::Integer,
            },
        ],
    };
    db.declare_view::<Rec>(&view).await.expect("declare");

    // filter + sort on projected columns: weight >= 200, desc
    let query = Query::new()
        .filter(Filter::gte("weight", 200i64))
        .sort_desc("weight");
    let rows = db.query_view("rec_view", &query).await.unwrap();
    // each row is [id, note, weight]
    let weights: Vec<i64> = rows
        .iter()
        .map(|r| match r[2] {
            StoredValue::Integer(n) => n,
            _ => -1,
        })
        .collect();
    assert_eq!(weights, vec![300, 200]);
}

#[derive(Debug, Clone, PartialEq, Scalar)]
struct TagId(Uuid);

#[derive(Debug, Clone, PartialEq, Collection)]
struct Tagged {
    #[bedrock(id)]
    id: TagId,
    tag: Option<String>,
}

#[tokio::test]
async fn ne_and_nin_include_null_rows() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Tagged>().await.expect("ensure");
    db.insert_all(vec![
        Tagged {
            id: TagId(Uuid::from_u128(1)),
            tag: Some("a".into()),
        },
        Tagged {
            id: TagId(Uuid::from_u128(2)),
            tag: Some("b".into()),
        },
        Tagged {
            id: TagId(Uuid::from_u128(3)),
            tag: None,
        },
    ])
    .await
    .expect("insert");

    // Mongo semantics: `$ne "a"` matches "b" AND the NULL row (SQL `<>` alone would drop it).
    let got = db
        .query::<Tagged>(&Query::new().filter(Filter::ne("tag", "a")))
        .await
        .unwrap();
    assert_eq!(got.len(), 2);
    assert!(got.iter().any(|t| t.tag.is_none()));

    // Same for `$nin ["a"]`.
    let got = db
        .query::<Tagged>(&Query::new().filter(Filter::not_in("tag", ["a".into()])))
        .await
        .unwrap();
    assert_eq!(got.len(), 2);
    assert!(got.iter().any(|t| t.tag.is_none()));
}

#[tokio::test]
async fn update_where_applies_set_and_unset() {
    let db = seeded().await;

    // $set: rewrite kind "a" -> "z" for both matching rows.
    let affected = db
        .update_where::<Rec>(&Filter::eq("kind", "a"), &Update::new().set("kind", "z"))
        .await
        .unwrap();
    assert_eq!(affected, 2);
    let got = db
        .query::<Rec>(&Query::new().filter(Filter::eq("kind", "z")))
        .await
        .unwrap();
    assert_eq!(got.len(), 2);
}

#[tokio::test]
async fn update_where_unset_clears_nullable_column() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Tagged>().await.expect("ensure");
    db.insert_all(vec![
        Tagged {
            id: TagId(Uuid::from_u128(1)),
            tag: Some("a".into()),
        },
        Tagged {
            id: TagId(Uuid::from_u128(2)),
            tag: Some("b".into()),
        },
    ])
    .await
    .expect("insert");

    // $unset clears tag on the "b" row; it then reads back as missing.
    let affected = db
        .update_where::<Tagged>(&Filter::eq("tag", "b"), &Update::new().unset("tag"))
        .await
        .unwrap();
    assert_eq!(affected, 1);
    let cleared = db
        .query::<Tagged>(&Query::new().filter(Filter::missing("tag")))
        .await
        .unwrap();
    assert_eq!(cleared.len(), 1);
    assert_eq!(cleared[0].id, TagId(Uuid::from_u128(2)));
}

#[tokio::test]
async fn update_where_refuses_invalid_columns() {
    let db = seeded().await;

    // $set on a column the collection does not have.
    let err = db
        .update_where::<Rec>(&Filter::eq("kind", "a"), &Update::new().set("nope", 1i64))
        .await
        .unwrap_err();
    assert!(matches!(err, bedrock::BedrockError::UnknownColumn { .. }));

    // $unset on a non-nullable column (the primary key).
    let err = db
        .update_where::<Rec>(&Filter::eq("kind", "a"), &Update::new().unset("id"))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        bedrock::BedrockError::UnsetNonNullable { .. }
    ));
}
