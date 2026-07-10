//! `serde_json::Value` as a document column, round-tripped through a live turso
//! database — including the distinction between a JSON `null` value and an
//! absent (`Option::None` → SQL NULL) column.

use bedrock::{Collection, Database, Scalar};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct BagId(Uuid);

#[derive(Debug, Clone, PartialEq, Collection)]
struct Bag {
    #[bedrock(id)]
    id: BagId,
    data: serde_json::Value,
    extra: Option<serde_json::Value>,
}

#[tokio::test]
async fn json_value_columns_roundtrip_through_the_engine() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Bag>().await.expect("ensure");

    let full = Bag {
        id: BagId(Uuid::from_u128(1)),
        data: serde_json::json!({ "k": [1, 2, 3], "nested": { "b": true } }),
        extra: Some(serde_json::json!({ "note": "hi" })),
    };
    // JSON `null` in `data` is a value; `extra: None` is an absent column (SQL NULL).
    let empty = Bag {
        id: BagId(Uuid::from_u128(2)),
        data: serde_json::Value::Null,
        extra: None,
    };
    db.insert_all(vec![full.clone(), empty.clone()])
        .await
        .expect("insert");

    assert_eq!(
        db.get::<Bag>(&BagId(Uuid::from_u128(1))).await.unwrap(),
        Some(full)
    );
    assert_eq!(
        db.get::<Bag>(&BagId(Uuid::from_u128(2))).await.unwrap(),
        Some(empty)
    );
}
