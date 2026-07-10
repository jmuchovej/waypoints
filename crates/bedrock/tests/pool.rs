use bedrock::{BedrockError, Config, Database, FlushMode};
use bedrock_core::{Filter, Query};
use bedrock_derive::{Collection, Scalar};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct ItemId(Uuid);

#[derive(Debug, Clone, PartialEq, Collection)]
struct Item {
    #[bedrock(id)]
    id: ItemId,
    #[bedrock(index)]
    kind: String,
}

fn item(n: u128, kind: &str) -> Item {
    Item {
        id: ItemId(Uuid::from_u128(n)),
        kind: kind.into(),
    }
}

async fn seeded(db: &Database) {
    db.ensure_collection::<Item>().await.expect("ensure");
    db.insert_all(&[item(1, "a"), item(2, "a"), item(3, "b")])
        .await
        .expect("insert");
}

#[tokio::test]
async fn reads_run_concurrently_with_a_held_write_connection() {
    let db = Database::open(":memory:").await.expect("open");
    seeded(&db).await;

    // Hold the single write connection in an open transaction...
    let tx = db.begin_write().await.expect("begin write");
    // ...and a pooled read still resolves (it uses a different connection).
    let all = db.all::<Item>().await.expect("read while writing");
    assert_eq!(all.len(), 3);
    tx.rollback().await.expect("rollback");
}

#[tokio::test]
async fn savepoint_rolls_back_within_an_outer_transaction() {
    let db = Database::open(":memory:").await.expect("open");
    seeded(&db).await;

    let mut tx = db.begin_write().await.expect("begin write");
    // Outer write: add "c".
    tx.execute(
        "INSERT INTO item (id, kind) VALUES (?, ?)",
        vec![turso_uuid(10), turso::Value::Text("c".into())],
    )
    .await
    .expect("outer insert");

    // Inner savepoint write that we then roll back.
    let sp = tx.savepoint().await.expect("savepoint");
    sp.execute(
        "INSERT INTO item (id, kind) VALUES (?, ?)",
        vec![turso_uuid(11), turso::Value::Text("d".into())],
    )
    .await
    .expect("inner insert");
    sp.rollback().await.expect("sp rollback");

    tx.commit().await.expect("commit");

    // Outer "c" survived; inner "d" was undone.
    let kinds = db
        .query::<Item>(&Query::new().filter(Filter::is_in("kind", ["c".into(), "d".into()])))
        .await
        .expect("query");
    assert_eq!(kinds.len(), 1);
    assert_eq!(kinds[0].kind, "c");
}

#[tokio::test]
async fn savepoint_commit_keeps_inner_writes() {
    let db = Database::open(":memory:").await.expect("open");
    seeded(&db).await;

    let mut tx = db.begin_write().await.expect("begin write");
    let sp = tx.savepoint().await.expect("savepoint");
    sp.execute(
        "INSERT INTO item (id, kind) VALUES (?, ?)",
        vec![turso_uuid(12), turso::Value::Text("e".into())],
    )
    .await
    .expect("inner insert");
    sp.commit().await.expect("sp release");
    tx.commit().await.expect("commit");

    let got = db
        .query::<Item>(&Query::new().filter(Filter::eq("kind", "e")))
        .await
        .expect("query");
    assert_eq!(got.len(), 1);
}

#[tokio::test]
async fn dropped_transaction_is_rolled_back_on_reacquire() {
    let db = Database::open(":memory:").await.expect("open");
    seeded(&db).await;

    // Begin, write, then drop without committing.
    {
        let tx = db.begin_write().await.expect("begin write");
        tx.execute(
            "INSERT INTO item (id, kind) VALUES (?, ?)",
            vec![turso_uuid(20), turso::Value::Text("z".into())],
        )
        .await
        .expect("insert");
        // `tx` dropped here — no commit.
    }

    // The next write acquisition rolls the abandoned transaction back, so the
    // "z" row never landed.
    db.insert(&item(21, "kept")).await.expect("later insert");
    let z = db
        .query::<Item>(&Query::new().filter(Filter::eq("kind", "z")))
        .await
        .expect("query");
    assert!(z.is_empty(), "abandoned insert must have rolled back");
}

#[tokio::test]
async fn read_transaction_sees_a_consistent_snapshot() {
    let db = Database::open(":memory:").await.expect("open");
    seeded(&db).await;

    let tx = db.begin_read().await.expect("begin read");
    let mut rows = tx
        .query("SELECT count(*) FROM item", ())
        .await
        .expect("count query");
    let n = match rows.next().await.expect("row").expect("some").get_value(0) {
        Ok(turso::Value::Integer(n)) => n,
        other => panic!("unexpected: {other:?}"),
    };
    drop(rows);
    assert_eq!(n, 3);
    tx.commit().await.expect("commit read");
}

#[tokio::test]
async fn close_refuses_further_operations() {
    let db = Database::open(":memory:").await.expect("open");
    seeded(&db).await;
    db.close().await;
    let err = db.all::<Item>().await.unwrap_err();
    assert!(matches!(err, BedrockError::Closed));
}

#[tokio::test]
async fn flush_on_a_memory_db_is_a_noop_success() {
    let db = Database::open(":memory:").await.expect("open");
    seeded(&db).await;
    // Passive checkpoint + fsync both succeed even without a WAL.
    db.flush().await.expect("passive flush");
    db.flush_with(FlushMode::Fsync).await.expect("fsync flush");
}

#[tokio::test]
async fn version_mismatch_is_refused_on_reopen() {
    let dir = std::env::temp_dir().join(format!("bedrock-pool-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("versioned.db");
    let path_str = path.to_str().expect("utf-8 path");
    let _ = std::fs::remove_file(&path);

    // Create at version 1.
    {
        let db = Database::open_with(path_str, Config::new().version(1))
            .await
            .expect("open v1");
        seeded(&db).await;
        db.flush().await.expect("flush");
        db.close().await;
    }

    // Reopening at a different version is refused.
    match Database::open_with(path_str, Config::new().version(2)).await {
        Err(BedrockError::IncompatibleVersion { want: 2, got: 1 }) => {}
        Err(other) => panic!("wrong error: {other:?}"),
        Ok(_) => panic!("version mismatch should have been refused"),
    }

    // Reopening at the same version works.
    let db = Database::open_with(path_str, Config::new().version(1))
        .await
        .expect("open v1 again");
    assert_eq!(db.count::<Item>().await.expect("count"), 3);

    let _ = std::fs::remove_file(&path);
}

fn turso_uuid(n: u128) -> turso::Value {
    // Matches bedrock's `uuid` encoding: the hyphenated string form.
    turso::Value::Text(Uuid::from_u128(n).to_string())
}
