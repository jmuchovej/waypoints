//! The native backend over the fixtures: goldens, determinism, and the
//! configuration gate. `UPDATE_GOLDENS=1` rewrites the goldens.

use std::{fs, path::PathBuf};

use plain_markdown::PlainMarkdown;
use serde_json::json;
use transload::{
    harness::{self, HarnessError, Run, golden, native},
    sdk::Importer,
};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn goldens() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("goldens")
}

fn entries() -> Vec<harness::Entry> {
    let manifest = PlainMarkdown::manifest().unwrap();
    harness::entries(&fixtures(), &manifest.source).unwrap()
}

fn run() -> Run {
    native::run::<PlainMarkdown>(&json!({"title_from": "heading"}), &entries()).unwrap()
}

#[test]
fn the_entries_are_the_markdown_files_sorted() {
    let names: Vec<_> = entries().into_iter().map(|entry| entry.name).collect();
    assert_eq!(names, ["alpha.md", "index.md", "notes/beta.md"]);
}

#[test]
fn the_goldens_match() {
    let run = run();
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        golden::write(&goldens(), &run).unwrap();
    }
    golden::check(&goldens(), &run).unwrap();
}

#[test]
fn two_runs_one_output() {
    assert_eq!(run().differences(&run()), []);
}

#[test]
fn a_changed_golden_names_its_entry() {
    let dir = tempfile::tempdir().unwrap();
    let run = run();
    golden::write(dir.path(), &run).unwrap();
    let path = dir.path().join("alpha.md.json");
    let mut bytes = fs::read(&path).unwrap();
    let index = bytes.iter().position(|byte| *byte == b'N').unwrap();
    bytes[index] = b'n';
    fs::write(&path, bytes).unwrap();

    let error = golden::check(dir.path(), &run).unwrap_err();
    let HarnessError::Mismatch { differences, .. } = &error else {
        panic!("{error}");
    };
    assert_eq!(differences.len(), 1);
    assert_eq!(differences[0].name, "alpha.md");
    assert!(
        differences[0].detail.contains("expected"),
        "{}",
        differences[0]
    );
}

#[test]
fn configuration_the_schema_rejects_never_reaches_init() {
    let error =
        native::run::<PlainMarkdown>(&json!({"title_from": "nope"}), &entries()).unwrap_err();
    assert!(matches!(error, HarnessError::Config(_)), "{error}");
    let error = native::run::<PlainMarkdown>(&json!({}), &entries()).unwrap_err();
    assert!(matches!(error, HarnessError::Config(_)), "{error}");
}
