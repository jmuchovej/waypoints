//! The crash victim: writes to a real file-backed database until the test
//! kills it (see `tests/crash_recovery.rs`).
//!
//! Usage: `crash-victim <db-path> <scenario>`. Scenarios drive writes and
//! print single-line progress markers on stdout; the spawning test kills this
//! process when it sees the marker for the scenario's kill point.
//!
//! Every marker is explicitly flushed: stdout here is a pipe, so it is
//! block-buffered, and an unflushed marker would leave the test waiting for
//! output that was already produced.

use bedrock::{Collection, Database};
use std::io::Write as _;

/// The rows the scenarios write. An `i64` id doubles as the acknowledgement
/// counter, so a marker like `ack 7` names the exact row it vouches for.
#[derive(Debug, Clone, Collection)]
struct CrashRow {
    #[bedrock(id)]
    id: i64,
    payload: String,
}

fn row(id: i64) -> CrashRow {
    CrashRow {
        id,
        payload: format!("row {id}"),
    }
}

/// Print one marker line and flush it through the pipe.
fn marker(line: &str) {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{line}").expect("write marker");
    out.flush().expect("flush marker");
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: crash-victim <db-path> <scenario>");
    let scenario = args
        .next()
        .expect("usage: crash-victim <db-path> <scenario>");

    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build runtime")
        .block_on(run(&path, &scenario));
}

async fn run(path: &str, scenario: &str) {
    let db = Database::open(path).await.expect("open database");
    db.ensure_collection::<CrashRow>()
        .await
        .expect("ensure collection");

    match scenario {
        "single-insert" => single_insert(&db).await,
        "open-transaction" => open_transaction(&db).await,
        "batch-insert" => batch_insert(&db).await,
        "idle" => idle(&db).await,
        other => panic!("unknown scenario {other:?}"),
    }
}

/// Rows in the scenarios' acknowledged baseline.
const BASELINE: i64 = 100;

/// Insert and acknowledge rows `1..=BASELINE`, then print `baseline <n>`.
/// One atomic batch: committed (and synced) before the marker acknowledges it,
/// without paying a per-row commit sync.
async fn baseline(db: &Database) {
    let rows: Vec<CrashRow> = (1..=BASELINE).map(row).collect();
    db.insert_all(rows).await.expect("baseline insert");
    marker(&format!("baseline {BASELINE}"));
}

/// Insert rows one at a time forever, printing `ack <id>` after each
/// acknowledged insert. The kill lands after some ack, while the next insert
/// is in flight.
async fn single_insert(db: &Database) {
    for id in 1.. {
        db.insert(row(id)).await.expect("insert");
        marker(&format!("ack {id}"));
    }
}

/// Acknowledge a baseline, then hold an explicit transaction open with
/// uncommitted rows and park. The kill lands on `tx-open`; none of the
/// uncommitted rows may survive recovery.
async fn open_transaction(db: &Database) {
    baseline(db).await;
    let tx = db.begin_write().await.expect("begin write");
    for id in (BASELINE + 1)..=(BASELINE + 10) {
        let sql = "INSERT INTO crash_row (id, payload) VALUES (?, ?)";
        let params = vec![
            turso::Value::Integer(id),
            turso::Value::Text(format!("uncommitted {id}")),
        ];
        tx.execute(sql, params).await.expect("uncommitted insert");
    }
    marker("tx-open");
    // Park forever, keeping `tx` alive and uncommitted until the kill.
    std::future::pending::<()>().await;
    drop(tx);
}

/// Rows in the interrupted batch.
const BATCH: i64 = 500;

/// Acknowledge a baseline, print `batch-start`, then push one multi-row
/// insert through the batch API so the kill lands inside its internal
/// `BEGIN IMMEDIATE` transaction. Prints `batch-done` if the kill loses the
/// race — the test's assertion is all-or-nothing either way.
async fn batch_insert(db: &Database) {
    baseline(db).await;
    let batch: Vec<CrashRow> = ((BASELINE + 1)..=(BASELINE + BATCH)).map(row).collect();
    marker("batch-start");
    db.insert_all(batch).await.expect("batch insert");
    marker("batch-done");
    std::future::pending::<()>().await;
}

/// Acknowledge a fixed population, print `idle`, and park without writing.
async fn idle(db: &Database) {
    baseline(db).await;
    marker("idle");
    std::future::pending::<()>().await;
}
