use bedrock::Database;
use bedrock_core::{Collection, ColumnType, StoredValue, ViewColumn, ViewScope, ViewSpec};
use bedrock_derive::{Collection, Document, Scalar};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct ItemId(Uuid);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Document)]
struct Payload {
    // a discriminant path, plus two projectable values
    kind: String,
    score: i64,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Collection)]
struct Item {
    #[bedrock(id)]
    id: ItemId,
    data: Payload,
}

fn item(n: u128, kind: &str, score: i64, label: &str) -> Item {
    Item {
        id: ItemId(Uuid::from_u128(n)),
        data: Payload {
            kind: kind.into(),
            score,
            label: label.into(),
        },
    }
}

fn kind_a_view() -> ViewSpec {
    ViewSpec {
        name: "item_a_view".into(),
        scope: Some(ViewScope {
            document: "data".into(),
            path: "kind".into(),
            equals: "a".into(),
        }),
        columns: vec![
            ViewColumn {
                name: "score".into(),
                document: "data".into(),
                path: "score".into(),
                column_type: ColumnType::Integer,
            },
            ViewColumn {
                name: "label".into(),
                document: "data".into(),
                path: "label".into(),
                column_type: ColumnType::Text,
            },
        ],
    }
}

#[tokio::test]
async fn materialized_view_projects_scopes_and_drops() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Item>().await.expect("ensure");

    db.insert_all(&[
        item(1, "a", 10, "x"),
        item(2, "a", 20, "y"),
        item(3, "b", 30, "z"),
    ])
    .await
    .expect("insert");

    // Declared after the writes → backfills from existing rows (the reliable,
    // recompute path). Idempotent re-declare must not error.
    db.declare_view::<Item>(&kind_a_view())
        .await
        .expect("declare");
    db.declare_view::<Item>(&kind_a_view())
        .await
        .expect("re-declare");

    // Scoped to kind "a" → 2 rows, each `[id, score, label]`.
    let mut rows = db.view_rows("item_a_view").await.expect("view rows");
    assert_eq!(rows.len(), 2);
    rows.sort_by_key(|r| match r[1] {
        StoredValue::Integer(n) => n,
        _ => -1,
    });
    assert_eq!(rows[0][1], StoredValue::Integer(10));
    assert_eq!(rows[0][2], StoredValue::Text("x".into()));
    assert_eq!(rows[1][1], StoredValue::Integer(20));
    assert_eq!(rows[1][2], StoredValue::Text("y".into()));

    db.drop_view("item_a_view").await.expect("drop");
    assert!(db.view_rows("item_a_view").await.is_err());
}
