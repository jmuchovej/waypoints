//! Crash recovery against the real engine: a victim process (`src/bin/
//! crash_victim.rs`) writes to a file-backed database and is SIGKILLed at a
//! marker-chosen point; the test reopens the database and asserts integrity
//! and content.
//!
//! Kills are marker-driven, never sleep-driven: the victim prints a flushed
//! line per meaningful step, and the test kills when the scenario's kill-point
//! marker arrives. Assertions are invariants that hold either side of the
//! kill's race with an in-flight write, not exact counts.
//!
//! Every test also carries task 5.6: the reopen is the ordinary open path with
//! no repair step, run while the killed process's `-wal` sidecar is still on
//! disk (asserted present first).

use bedrock::{Collection, Database};
use std::io::BufRead as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Mirror of the victim's collection — same name and shape, so the reopened
/// database reads the victim's rows. Keep in sync with `crash_victim.rs`.
#[derive(Debug, Clone, Collection)]
struct CrashRow {
    #[bedrock(id)]
    id: i64,
    payload: String,
}

/// Rows in the victim's acknowledged baseline (`BASELINE` in the victim).
const BASELINE: i64 = 100;
/// Rows in the victim's interrupted batch (`BATCH` in the victim).
const BATCH: i64 = 500;

/// How long to wait for any single marker before failing the test. An upper
/// bound for a loaded CI machine, not a pace: markers normally arrive in
/// milliseconds.
const MARKER_TIMEOUT: Duration = Duration::from_secs(30);

/// A spawned victim: piped stdout drained by a reader thread, killed on drop
/// so a failing assertion never leaks a process.
struct Victim {
    child: std::process::Child,
    lines: mpsc::Receiver<String>,
}

impl Victim {
    fn spawn(db_path: &Path, scenario: &str) -> Victim {
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_crash-victim"))
            .arg(db_path)
            .arg(scenario)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("spawn crash victim");
        let stdout = child.stdout.take().expect("piped stdout");

        // Reader thread: the channel gives the test `recv_timeout`, so a
        // missing marker fails the test instead of hanging it. The thread
        // exits when the kill closes the pipe.
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if send.send(line).is_err() {
                    break;
                }
            }
        });

        Victim { child, lines }
    }

    /// Read markers until one starts with `prefix`, returning it.
    fn wait_for(&mut self, prefix: &str) -> String {
        loop {
            let line = self
                .lines
                .recv_timeout(MARKER_TIMEOUT)
                .unwrap_or_else(|e| panic!("marker {prefix:?} never arrived: {e}"));
            if line.starts_with(prefix) {
                return line;
            }
        }
    }

    /// Terminate without giving the victim a chance to clean up (SIGKILL on
    /// Unix, `TerminateProcess` on Windows), and reap it.
    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Every marker still in flight after the kill: lines the victim printed
    /// (and flushed) before dying that the test had not read yet. Ends when
    /// the reader thread drains the closed pipe and drops its sender.
    fn drain(&mut self) -> Vec<String> {
        let mut rest = Vec::new();
        while let Ok(line) = self.lines.recv_timeout(MARKER_TIMEOUT) {
            rest.push(line);
        }
        rest
    }
}

impl Drop for Victim {
    fn drop(&mut self) {
        self.kill();
    }
}

/// A fresh per-test working directory under the system temp dir, keyed by pid
/// *and* test name: the harness runs tests as parallel threads of one
/// process, so the pid alone would collide.
fn scratch_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bedrock-crash-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Reopen the killed victim's database through the ordinary open path and
/// verify the engine reports a sound file. Asserts the `-wal` sidecar the
/// kill left behind is present first — recovery from it is the thing under
/// test — and that the reopened handle carries no durability fault.
async fn reopen_and_check(db_path: &Path) -> Database {
    let wal = db_path.with_extension("db-wal");
    assert!(
        wal.exists(),
        "killed victim should leave a WAL sidecar at {}",
        wal.display()
    );

    let db = Database::open(db_path.to_str().expect("utf-8 path"))
        .await
        .expect("reopen after kill through the ordinary open path");
    assert!(
        db.durability_fault().is_none(),
        "reopen must not be faulted"
    );
    assert_eq!(
        db.integrity_check().await.expect("integrity check"),
        vec!["ok".to_string()],
        "integrity after recovery"
    );
    db
}

/// The reopened rows' ids, sorted.
async fn ids(db: &Database) -> Vec<i64> {
    let mut ids: Vec<i64> = db
        .all::<CrashRow>()
        .await
        .expect("read recovered rows")
        .into_iter()
        .map(|r| r.id)
        .collect();
    ids.sort_unstable();
    ids
}

#[tokio::test]
async fn kill_during_single_insert_keeps_every_acknowledged_row() {
    let dir = scratch_dir("single_insert");
    let db_path = dir.join("victim.db");

    let mut victim = Victim::spawn(&db_path, "single-insert");
    // Let it make real progress, then kill while the next insert is in
    // flight: the ack for row N prints after N committed, so the victim is
    // already inside insert N+1 when the kill lands.
    let mut last_acked: i64 = 0;
    while last_acked < 20 {
        let ack = victim.wait_for("ack ");
        last_acked = ack["ack ".len()..].parse().expect("ack row id");
    }
    victim.kill();
    // The victim kept inserting between our read of that ack and the kill
    // landing; anything it acknowledged in that window is already in the
    // pipe. Drain it — "last acknowledged" means the victim's last ack, not
    // the last one the test happened to have read.
    for line in victim.drain() {
        if let Some(n) = line.strip_prefix("ack ") {
            last_acked = n.parse().expect("ack row id");
        }
    }

    let db = reopen_and_check(&db_path).await;
    let got = ids(&db).await;
    // Every acknowledged row must be present; the in-flight insert may have
    // committed before its ack printed, so tolerate exactly one row beyond.
    assert!(
        got.len() as i64 >= last_acked,
        "acknowledged rows lost: {} recovered < {last_acked} acked",
        got.len()
    );
    assert!(
        got.len() as i64 <= last_acked + 1,
        "more than one unacknowledged row survived: {} recovered",
        got.len()
    );
    let expect: Vec<i64> = (1..=got.len() as i64).collect();
    assert_eq!(got, expect, "recovered rows must be the contiguous prefix");

    db.close().await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn kill_with_open_transaction_recovers_baseline_only() {
    let dir = scratch_dir("open_transaction");
    let db_path = dir.join("victim.db");

    let mut victim = Victim::spawn(&db_path, "open-transaction");
    victim.wait_for("tx-open");
    victim.kill();

    let db = reopen_and_check(&db_path).await;
    let expect: Vec<i64> = (1..=BASELINE).collect();
    assert_eq!(
        ids(&db).await,
        expect,
        "exactly the baseline: every pre-transaction row, none of the uncommitted ones"
    );

    db.close().await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn kill_during_batch_insert_is_all_or_nothing() {
    let dir = scratch_dir("batch_insert");
    let db_path = dir.join("victim.db");

    let mut victim = Victim::spawn(&db_path, "batch-insert");
    victim.wait_for("batch-start");
    // Deliberately racy: the kill lands somewhere inside (or, rarely, before
    // or after) the batch's internal `BEGIN IMMEDIATE` transaction. The
    // assertion is stable under any timing.
    victim.kill();

    let db = reopen_and_check(&db_path).await;
    let got = ids(&db).await;
    let baseline: Vec<i64> = (1..=BASELINE).collect();
    let full: Vec<i64> = (1..=BASELINE + BATCH).collect();
    assert!(
        got == baseline || got == full,
        "batch must be all-or-nothing: recovered {} rows",
        got.len()
    );

    db.close().await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn kill_while_idle_recovers_contents_exactly() {
    let dir = scratch_dir("idle");
    let db_path = dir.join("victim.db");

    let mut victim = Victim::spawn(&db_path, "idle");
    victim.wait_for("idle");
    victim.kill();

    let db = reopen_and_check(&db_path).await;
    let expect: Vec<i64> = (1..=BASELINE).collect();
    assert_eq!(ids(&db).await, expect, "idle territory recovers exactly");

    db.close().await;
    let _ = std::fs::remove_dir_all(&dir);
}
