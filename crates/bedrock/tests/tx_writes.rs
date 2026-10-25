use bedrock::{Collection, Database, Scalar};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct AId(Uuid);

#[derive(Debug, Clone, PartialEq, Collection)]
struct A {
    #[bedrock(id)]
    id: AId,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Scalar)]
struct BId(Uuid);

#[derive(Debug, Clone, PartialEq, Collection)]
struct B {
    #[bedrock(id)]
    id: BId,
    weight: i64,
}

async fn open() -> Database {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<A>().await.expect("a");
    db.ensure_collection::<B>().await.expect("b");
    db
}

fn a(n: u128) -> A {
    A {
        id: AId(Uuid::from_u128(n)),
        label: format!("a{n}"),
    }
}

fn b(n: u128) -> B {
    B {
        id: BId(Uuid::from_u128(n)),
        weight: n as i64,
    }
}

#[tokio::test]
async fn a_rolled_back_transaction_writes_nothing_to_any_collection() {
    let db = open().await;
    let tx = db.begin_write().await.expect("begin");
    tx.insert(a(1)).await.expect("insert a");
    tx.insert(b(2)).await.expect("insert b");
    tx.rollback().await.expect("rollback");

    assert_eq!(db.count::<A>().await.expect("count a"), 0);
    assert_eq!(db.count::<B>().await.expect("count b"), 0);
}

#[tokio::test]
async fn a_committed_transaction_writes_every_collection() {
    let db = open().await;
    let tx = db.begin_write().await.expect("begin");
    tx.insert(a(1)).await.expect("insert a");
    tx.insert(b(2)).await.expect("insert b");
    tx.upsert(A {
        label: "replaced".into(),
        ..a(1)
    })
    .await
    .expect("upsert a");
    tx.commit().await.expect("commit");

    let stored = db.get::<A>(&AId(Uuid::from_u128(1))).await.expect("get");
    assert_eq!(stored.map(|row| row.label).as_deref(), Some("replaced"));
    assert_eq!(db.count::<B>().await.expect("count b"), 1);
}
