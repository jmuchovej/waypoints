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
//! nextest resolves both closures itself, so this only names the roots. The
//! whole workspace still *compiles* — a filterset bounds what runs, not what is
//! built — which keeps feature resolution identical to a release build and is
//! why the cost is addressed with a compilation cache rather than by narrowing
//! the build.
//!
//! Mapping changed *files* to crates is the part nextest cannot do, and is what
//! this module computes. An empty plan means no Rust crate is affected — a
//! website- or docs-only change — and the caller skips testing entirely rather
//! than falling back to everything.
//!
//! [nextest filterset]: https://nexte.st/docs/filtersets/

use anyhow::{Context, Result};
use clap::Parser;
use std::{collections::BTreeSet, process::Command};

/// Crates the release artifacts are built from. Their in-tree dependencies come
/// along via `deps()`, so this lists only the roots.
///
/// `Waypoints` carries no tests of its own — it is the thin Tauri shell — but
/// naming it pulls in everything it delegates to.
///
/// Deliberately not `all()`. `server`, `transload`, `importer-sdk`, the importer
/// plugins and `xtask` are maintained here but linked into nothing that ships,
/// so a half-finished test in one of them must not block an app release. Their
/// tests still run on any pull request that touches them.
const RELEASE_ROOTS: [&str; 2] = ["Waypoints", "cartograph"];

/// Paths that invalidate every crate, so a change to one tests the lot.
///
/// Deliberately narrow. `.github/` is *not* here: a workflow edit cannot change
/// how Rust behaves, and treating it as global would test the whole workspace
/// every time a website job is retuned.
const GLOBAL_PATHS: [&str; 4] = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/"];

#[derive(Parser)]
pub(crate) struct TestPlanArgs {
    /// Plan the pre-release suite instead of a change-scoped one.
    #[arg(long)]
    release: bool,

    /// Revision to diff against. Defaults to the pull request's merge base, or
    /// the previous commit on a push.
    #[arg(long)]
    base: Option<String>,
}

pub(crate) fn run(args: TestPlanArgs) -> Result<()> {
    if args.release {
        println!(
            "{}",
            union(RELEASE_ROOTS.iter().map(|root| format!("deps({root})")))
        );
        return Ok(());
    }

    let base = match args.base {
        Some(base) => base,
        None => default_base()?,
    };

    let metadata = crate::workspace::load_workspace()?;
    let root = metadata.workspace_root.as_std_path();
    let directories: Vec<(String, String)> = metadata
        .workspace_packages()
        .iter()
        .filter_map(|package| {
            let dir = package.manifest_path.as_std_path().parent()?;
            let relative = dir.strip_prefix(root).ok()?;
            Some((package.name.to_string(), relative.to_string_lossy().into()))
        })
        .collect();

    println!("{}", plan(&changed_files(&base)?, &directories));
    Ok(())
}

/// The filterset for a set of changed files, or empty when nothing Rust-facing
/// changed.
fn plan(changed: &[String], directories: &[(String, String)]) -> String {
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
fn default_base() -> Result<String> {
    match std::env::var("GITHUB_BASE_REF") {
        Ok(base) if !base.is_empty() => git(&["merge-base", &format!("origin/{base}"), "HEAD"]),
        _ => Ok("HEAD~1".to_owned()),
    }
}

fn changed_files(base: &str) -> Result<Vec<String>> {
    let out = git(&["diff", "--name-only", base, "HEAD"])?;
    Ok(out.lines().map(str::to_owned).collect())
}

fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to run `git`")?;

    if !output.status.success() {
        anyhow::bail!(
            "`git {}` failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8(output.stdout)
        .context("`git` emitted invalid UTF-8")?
        .trim()
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directories() -> Vec<(String, String)> {
        [
            ("Waypoints", "waypoints/src-tauri"),
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
        let filterset = union(RELEASE_ROOTS.iter().map(|root| format!("deps({root})")));
        assert_eq!(filterset, "deps(Waypoints) + deps(cartograph)");
    }

    /// `xtask` generates the workflows and plans the tests, so a change to it
    /// must test it — nothing depends on it, so nothing else would.
    #[test]
    fn a_change_to_xtask_tests_xtask() {
        assert_eq!(planned(&["tooling/xtask/src/main.rs"]), "rdeps(xtask)");
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

    #[test]
    fn a_toolchain_change_tests_everything() {
        for path in [
            "Cargo.lock",
            "Cargo.toml",
            "rust-toolchain.toml",
            ".cargo/config.toml",
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
