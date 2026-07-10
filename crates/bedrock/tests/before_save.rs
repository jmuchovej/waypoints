//! The `#[bedrock(before_save = …)]` hook: a Rust function run just before a
//! whole-entity write, transforming the value the engine stores. Verified
//! against a live database — the returned entity AND the row read back both
//! reflect the hook, for insert, insert_all, and update.

use bedrock::{Collection, Database, Scalar, WriteOp};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct DocId(Uuid);

/// Bump the revision on every write and record which write it was — the kind of
/// derived/audit bookkeeping a `before_save` hook exists for.
fn stamp(mut doc: Doc, op: WriteOp) -> Doc {
    doc.revision += 1;
    doc.last = match op {
        WriteOp::Insert => "insert".into(),
        WriteOp::Update => "update".into(),
    };
    doc
}

#[derive(Debug, Clone, PartialEq, Collection)]
#[bedrock(before_save = stamp)]
struct Doc {
    #[bedrock(id)]
    id: DocId,
    revision: i64,
    last: String,
}

fn doc(n: u128) -> Doc {
    Doc {
        id: DocId(Uuid::from_u128(n)),
        revision: 0,
        last: String::new(),
    }
}

#[tokio::test]
async fn insert_runs_the_hook_on_returned_and_stored_entity() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Doc>().await.expect("ensure");

    // The returned entity reflects the hook (revision 0→1, last = "insert").
    let stored = db.insert(doc(1)).await.expect("insert");
    assert_eq!(stored.revision, 1);
    assert_eq!(stored.last, "insert");

    // The row read back from the engine matches the returned entity exactly.
    let read = db.get::<Doc>(&DocId(Uuid::from_u128(1))).await.unwrap();
    assert_eq!(read, Some(stored));
}

#[tokio::test]
async fn update_runs_the_hook_with_update_op() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Doc>().await.expect("ensure");
    db.insert(doc(2)).await.expect("insert"); // revision 1, last "insert"

    let current = db
        .get::<Doc>(&DocId(Uuid::from_u128(2)))
        .await
        .unwrap()
        .unwrap();
    assert!(db.update(current).await.unwrap());

    // The Update hook bumped revision 1→2 and re-stamped `last`.
    let after = db
        .get::<Doc>(&DocId(Uuid::from_u128(2)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.revision, 2);
    assert_eq!(after.last, "update");
}

#[tokio::test]
async fn insert_all_returns_stamped_entities() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Doc>().await.expect("ensure");

    let stored = db.insert_all(vec![doc(10), doc(11)]).await.expect("batch");
    assert!(stored.iter().all(|d| d.revision == 1 && d.last == "insert"));
    assert_eq!(db.count::<Doc>().await.unwrap(), 2);
}
