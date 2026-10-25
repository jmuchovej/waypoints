//! Decides which tests a CI run needs.
//!
//! Two questions, one answer shape — a [nextest filterset]:
//!
//! - **Pre-release**: everything that ships — `Waypoints` and `cartograph` plus
//!   their in-tree dependencies. Not narrowed by the diff: a release runs the
//!   whole suite for those crates regardless of what changed.
//! - **Otherwise**: only the crates a change can break — the changed crate and
//!   everything in-tree that depends on it.
//!
//! nextest resolves both closures itself, so this names only the roots and
//! maps changed *files* to the crates that own them. A filterset bounds what
//! runs, not what is built. An empty plan means no Rust crate is affected.
//!
//! [nextest filterset]: https://nexte.st/docs/filtersets/

use crate::tools::{git, github};
use anyhow::Result;
use cargo_metadata::Metadata;
use std::collections::BTreeSet;

/// Crates the release artifacts are built from; `deps()` pulls in their in-tree
/// dependencies, so only the roots are listed.
///
/// Anything absent — `transload`, `importer-sdk`, the importer plugins,
/// `xtask` — ships in nothing and so cannot gate a release.
const RELEASE_ROOTS: [&str; 2] = ["Waypoints", "cartograph"];

/// Paths that invalidate every crate. `.github/` is excluded — a workflow edit
/// cannot change how Rust behaves.
const GLOBAL_PATHS: [&str; 4] = [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".config/cargo.toml",
];

/// The pre-release suite: the roots and everything they pull in.
pub fn release_plan() -> String {
    union(RELEASE_ROOTS.iter().map(|root| format!("deps({root})")))
}

/// The filterset for a set of changed files, or empty when nothing Rust-facing
/// changed.
pub fn plan(changed: &[String], directories: &[(String, String)]) -> String {
    if changed
        .iter()
        .any(|path| GLOBAL_PATHS.iter().any(|global| path.starts_with(global)))
    {
        return "all()".to_owned();
    }

    let touched: BTreeSet<&str> = changed
        .iter()
        .filter_map(|path| owning_package(path, directories))
        .collect();

    union(touched.iter().map(|package| format!("rdeps({package})")))
}

/// Every workspace package with its directory relative to the workspace
/// root — what [`plan`] maps changed paths through.
pub fn package_directories(metadata: &Metadata) -> Vec<(String, String)> {
    let root = metadata.workspace_root.as_std_path();
    metadata
        .workspace_packages()
        .iter()
        .filter_map(|package| {
            let dir = package.manifest_path.as_std_path().parent()?;
            let relative = dir.strip_prefix(root).ok()?;
            Some((package.name.to_string(), relative.to_string_lossy().into()))
        })
        .collect()
}

/// The workspace package a repo-relative path belongs to, if any.
///
/// Longest prefix wins, so a nested crate is not shadowed by an ancestor.
fn owning_package<'a>(path: &str, directories: &'a [(String, String)]) -> Option<&'a str> {
    directories
        .iter()
        .filter(|(_, dir)| path.starts_with(&format!("{dir}/")))
        .max_by_key(|(_, dir)| dir.len())
        .map(|(name, _)| name.as_str())
}

/// Joins terms into a filterset. Empty in, empty out — meaning "test nothing".
fn union(terms: impl Iterator<Item = String>) -> String {
    terms.collect::<Vec<_>>().join(" + ")
}

/// The merge base on a pull request, the previous commit on a push.
pub fn default_base() -> Result<String> {
    match github::env("GITHUB_BASE_REF") {
        Ok(base) if !base.is_empty() => {
            git::run(&["merge-base", &format!("origin/{base}"), "HEAD"])
        }
        _ => Ok("HEAD~1".to_owned()),
    }
}

pub fn changed_files(base: &str) -> Result<Vec<String>> {
    let out = git::run(&["diff", "--name-only", base, "HEAD"])?;
    Ok(out.lines().map(str::to_owned).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directories() -> Vec<(String, String)> {
        [
            ("Waypoints", "src-tauri"),
            ("cartograph", "crates/cartograph"),
            ("store", "crates/store"),
            ("domain", "crates/domain"),
            ("xtask", "tooling/xtask"),
        ]
        .into_iter()
        .map(|(n, d)| (n.to_owned(), d.to_owned()))
        .collect()
    }

    fn planned(changed: &[&str]) -> String {
        let changed: Vec<String> = changed.iter().map(|p| (*p).to_owned()).collect();
        plan(&changed, &directories())
    }

    #[test]
    fn a_release_covers_the_roots_and_their_dependencies() {
        assert_eq!(release_plan(), "deps(Waypoints) + deps(cartograph)");
    }

    /// `xtask` generates the workflows and plans the tests, so a change to it
    /// must test it — nothing depends on it, so nothing else would.
    #[test]
    fn a_change_to_xtask_tests_xtask() {
        assert_eq!(planned(&["tooling/xtask/src/lib.rs"]), "rdeps(xtask)");
    }

    #[test]
    fn a_crate_change_tests_it_and_its_dependants() {
        assert_eq!(planned(&["crates/domain/src/lib.rs"]), "rdeps(domain)");
    }

    #[test]
    fn several_crates_union_their_closures() {
        assert_eq!(
            planned(&["crates/domain/src/lib.rs", "crates/store/src/db.rs"]),
            "rdeps(domain) + rdeps(store)"
        );
    }

    #[test]
    fn repeated_crates_are_deduplicated() {
        assert_eq!(
            planned(&["crates/store/src/a.rs", "crates/store/src/b.rs"]),
            "rdeps(store)"
        );
    }

    /// `.config/cargo.toml` rather than `.cargo/config.toml`: the latter is a
    /// symlink to the former, which git tracks as a symlink blob. Editing the
    /// cargo config therefore reports the real path as changed and leaves the
    /// link untouched, so the real path is what a plan has to widen on.
    #[test]
    fn a_toolchain_change_tests_everything() {
        for path in [
            "Cargo.lock",
            "Cargo.toml",
            "rust-toolchain.toml",
            ".config/cargo.toml",
        ] {
            assert_eq!(planned(&[path]), "all()", "{path} should widen");
        }
    }

    #[test]
    fn a_global_change_outranks_a_scoped_one() {
        assert_eq!(
            planned(&["crates/domain/src/lib.rs", "Cargo.lock"]),
            "all()"
        );
    }

    /// The website has no bearing on the Rust workspace, so it must test
    /// nothing at all — not fall back to everything.
    #[test]
    fn a_website_change_tests_nothing() {
        assert!(planned(&["website/app/pages/index.vue"]).is_empty());
        assert!(planned(&["website/package.json"]).is_empty());
    }

    #[test]
    fn docs_and_tooling_outside_the_workspace_test_nothing() {
        assert!(planned(&["docs/architecture.md"]).is_empty());
        assert!(planned(&["fastlane/Fastfile"]).is_empty());
        assert!(planned(&[".github/workflows/pull-request.yaml"]).is_empty());
    }

    #[test]
    fn an_empty_diff_tests_nothing() {
        assert!(planned(&[]).is_empty());
    }

    /// A website change alongside a crate change still tests the crate.
    #[test]
    fn unrelated_changes_do_not_mask_a_crate_change() {
        assert_eq!(
            planned(&["website/app/index.vue", "crates/store/src/db.rs"]),
            "rdeps(store)"
        );
    }

    #[test]
    fn a_sibling_directory_is_not_a_match() {
        assert!(planned(&["crates/domain-other/src/lib.rs"]).is_empty());
    }
}
