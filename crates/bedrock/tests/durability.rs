//! Regression guard for the durability configuration: the effective journal
//! mode and synchronous level are asserted from a live connection against the
//! real engine, so weakening a default fails here instead of shipping.
//!
//! This suite is also the release gate's teeth: it must pass before a release
//! is cut and is re-run as a precondition of accepting any storage-engine
//! upgrade (the engine is pre-v1 and deliberately pinned — see the pin in the
//! workspace `Cargo.toml`).

use bedrock::{BedrockError, Collection, Config, Database, FlushMode, JournalMode, SyncMode};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Collection)]
struct Guard {
    #[bedrock(id)]
    id: i64,
    note: String,
}

fn row(id: i64) -> Guard {
    Guard {
        id,
        note: format!("row {id}"),
    }
}

/// Per-test scratch dir: pid + test name, matching the crash suite's pattern.
fn scratch_dir(test: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("bedrock-durability-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

#[tokio::test]
async fn default_config_reads_back_wal_and_full() {
    let dir = scratch_dir("defaults");
    let path = dir.join("guard.db");

    let db = Database::open(path.to_str().expect("utf-8 path"))
        .await
        .expect("open");
    let state = db.durability().await.expect("readback");
    assert_eq!(state.journal_mode, JournalMode::WAL);
    assert_eq!(state.synchronous, SyncMode::Full);
    assert!(db.durability_fault().is_none());

    db.close().await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn explicitly_weakened_synchronous_is_honoured_and_reported() {
    let dir = scratch_dir("weakened");
    let path = dir.join("guard.db");

    // Weakening through the typed knob is a deliberate configuration, not a
    // fault: the guard asserts effective state, not a hardcoded FULL.
    let db = Database::open_with(
        path.to_str().expect("utf-8 path"),
        Config::new().synchronous(SyncMode::Normal),
    )
    .await
    .expect("open weakened");
    let state = db.durability().await.expect("readback");
    assert_eq!(state.synchronous, SyncMode::Normal);
    assert_eq!(state.journal_mode, JournalMode::WAL);
    assert!(
        db.durability_fault().is_none(),
        "a honoured level is no fault"
    );

    // And the handle is fully writable.
    db.ensure_collection::<Guard>().await.expect("ensure");
    db.insert(row(1)).await.expect("insert");

    db.close().await;
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn memory_database_opens_with_no_journal_guarantee_and_flushes_as_noops() {
    let db = Database::open(":memory:").await.expect("open memory");
    // No journal-mode assertion for `:memory:` — there is no on-disk journal
    // and no guarantee. SyncMode is per-connection state and still applies.
    assert!(db.durability_fault().is_none());
    let state = db.durability().await.expect("readback");
    assert_eq!(state.synchronous, SyncMode::Full);

    db.ensure_collection::<Guard>().await.expect("ensure");
    db.insert(row(1)).await.expect("insert");
    for mode in [
        FlushMode::Fsync,
        FlushMode::Passive,
        FlushMode::Full,
        FlushMode::Restart,
    ] {
        db.flush_with(mode)
            .await
            .expect("flush succeeds as a no-op");
    }
    db.close().await;
}

#[tokio::test]
async fn rejected_caller_pragma_stays_best_effort() {
    let db = Database::open_with(
        ":memory:",
        Config::new().pragma("definitely_not_a_pragma", "1"),
    )
    .await
    .expect("a rejected caller pragma must not fail the open");
    assert!(db.durability_fault().is_none());
    db.ensure_collection::<Guard>().await.expect("ensure");
    db.insert(row(1)).await.expect("insert");
    db.close().await;
}

#[tokio::test]
async fn durability_mismatch_opens_read_only_with_a_retrievable_fault() {
    let dir = scratch_dir("mismatch");
    let path = dir.join("guard.db");
    let path = path.to_str().expect("utf-8 path");

    // Populate the territory under the default (sound) configuration.
    {
        let db = Database::open(path).await.expect("open sound");
        db.ensure_collection::<Guard>().await.expect("ensure");
        db.insert_all((1..=3).map(row).collect())
            .await
            .expect("seed rows");
        db.close().await;
    }

    // Reopen with a caller pragma dragging the effective level away from the
    // verified `Config::synchronous` — the open-time readback must catch it.
    let db = Database::open_with(path, Config::new().pragma("synchronous", "OFF"))
        .await
        .expect("mismatched open must still succeed");

    // The structured fault is retrievable state on the handle.
    let fault = db.durability_fault().expect("fault recorded").clone();
    assert_eq!(fault.pragma, "synchronous");
    assert_eq!(fault.requested, "FULL");
    assert_eq!(fault.effective, "OFF");
    let report = fault.report();
    assert!(
        report.contains(bedrock::ENGINE_VERSION),
        "report names the resolved engine version"
    );

    // Reads are preserved: the notes are on disk and readable.
    assert_eq!(db.count::<Guard>().await.expect("count"), 3);
    assert_eq!(db.all::<Guard>().await.expect("read")[0], row(1));

    // Writes are refused, each refusal carrying the fault.
    for err in [
        db.insert(row(4)).await.expect_err("insert must be refused"),
        db.begin_write()
            .await
            .err()
            .expect("begin_write must be refused"),
    ] {
        match err {
            BedrockError::DurabilityMismatch(carried) => assert_eq!(carried, fault),
            other => panic!("wrong refusal error: {other:?}"),
        }
    }
    // And the refused write changed nothing.
    assert_eq!(db.count::<Guard>().await.expect("count"), 3);

    db.close().await;
    let _ = std::fs::remove_dir_all(&dir);
}
