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
  ignore-revs  = ".git-blame-ignore-revs"
  bot-subjects = [ "chore(release):" ]

[group.waypoints]
  members  = [ "src-tauri" ]
  anchor   = "src-tauri"
  tag      = "v{version}"
  breaking = "trailer:Covenant: break"
  floors   = [ "covenant-fixtures" ]
  fixtures = "covenant/fixtures/waypoints/"
  minted   = [ "cut" ]

[enforce]
  commits        = "conventional"
  summary-type   = "summary"
  version-writes = "botswain"
  fixtures       = "append-only"
  schema         = "current"
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

fn assert_refuses_naming_the_crate(args: &[&str]) {
    let workspace = workspace_with_an_unnamed_member();
    let output = xtask(workspace.path(), args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{args:?} ran: {stderr}");
    assert!(stderr.contains("crates/importer"), "{args:?}: {stderr}");
}

#[test]
fn the_release_planner_refuses_an_unnamed_crate() {
    assert_refuses_naming_the_crate(&["release-plan", "cut", "--dry-run"]);
}

#[test]
fn the_nightly_planner_refuses_an_unnamed_crate() {
    assert_refuses_naming_the_crate(&["nightly-plan", "--build-only"]);
}
