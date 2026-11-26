// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Both planners refuse a workspace the group file does not describe, before
//! they read any history: the fixture workspace has no git at all, so reaching
//! a `git` call would report that failure instead of naming the crate.

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

const GROUP_FILE: &str = r#"
[bookkeeping]
  paths        = [ "nix/releases/" ]
  bot-subjects = [ "chore(release):" ]

[group.waypoints]
  version  = "0.1.0-nightly"
  members  = [ "src-tauri" ]
  tag      = "v{version}"
  fixtures = "covenant/fixtures/waypoints/"
"#;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn member(root: &Path, directory: &str, name: &str) {
    write(
        root,
        &format!("{directory}/Cargo.toml"),
        &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0-nightly\"\nedition = \"2024\"\n"),
    );
    write(root, &format!("{directory}/src/lib.rs"), "");
}

/// A workspace whose group file names `src-tauri` and not `crates/importer`.
fn workspace_with_an_unnamed_member() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nresolver = \"3\"\nmembers = [ \"crates/importer\", \"src-tauri\" ]\n",
    );
    member(root, "src-tauri", "Waypoints");
    member(root, "crates/importer", "importer");
    write(root, "tooling/release-groups.toml", GROUP_FILE);
    write(root, "covenant/fixtures/waypoints/README.md", "");
    write(root, "nix/releases/stable.json", "");
    write(root, ".git-blame-ignore-revs", "");
    dir
}

fn xtask(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("xtask runs")
}

/// A workspace whose app manifest declares another version than the group
/// file does for its group.
fn workspace_with_a_stale_projection() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nresolver = \"3\"\nmembers = [ \"src-tauri\" ]\n",
    );
    member(root, "src-tauri", "Waypoints");
    write(
        root,
        "tooling/release-groups.toml",
        &GROUP_FILE.replace("0.1.0-nightly", "0.2.0-nightly"),
    );
    write(root, "covenant/fixtures/waypoints/README.md", "");
    write(root, "nix/releases/stable.json", "");
    write(root, ".git-blame-ignore-revs", "");
    dir
}

fn assert_refuses_naming(workspace: &TempDir, args: &[&str], names: &[&str]) {
    let output = xtask(workspace.path(), args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{args:?} ran: {stderr}");
    for name in names {
        assert!(stderr.contains(name), "{args:?} names no {name}: {stderr}");
    }
}

fn assert_refuses_naming_the_crate(args: &[&str]) {
    assert_refuses_naming(
        &workspace_with_an_unnamed_member(),
        args,
        &["crates/importer"],
    );
}

/// Neither planner plans from a projection `version-integrity` would refuse:
/// the manifest and the file are both named.
fn assert_refuses_naming_the_manifest_and_the_file(args: &[&str]) {
    assert_refuses_naming(
        &workspace_with_a_stale_projection(),
        args,
        &[
            "src-tauri/Cargo.toml",
            "0.1.0-nightly",
            "0.2.0-nightly",
            "tooling/release-groups.toml",
        ],
    );
}

#[test]
fn the_release_planner_refuses_an_unnamed_crate() {
    assert_refuses_naming_the_crate(&["release", "plan", "cut", "--dry-run"]);
}

#[test]
fn the_nightly_planner_refuses_an_unnamed_crate() {
    assert_refuses_naming_the_crate(&["release", "bump", "--channel", "nightly", "--dry-run"]);
}

#[test]
fn the_release_planner_refuses_a_stale_projection() {
    assert_refuses_naming_the_manifest_and_the_file(&["release", "plan", "cut", "--dry-run"]);
}

#[test]
fn the_nightly_planner_refuses_a_stale_projection() {
    assert_refuses_naming_the_manifest_and_the_file(&[
        "release",
        "bump",
        "--channel",
        "nightly",
        "--dry-run",
    ]);
}
