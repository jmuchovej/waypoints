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

use anyhow::Result;
use clap::Parser;
use std::collections::BTreeSet;

use crate::shell;

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

#[derive(Parser)]
pub struct TestPlanArgs {
    /// Plan the pre-release suite instead of a change-scoped one.
    #[arg(long)]
    release: bool,

    /// Revision to diff against. Defaults to the pull request's merge base, or
    /// the previous commit on a push.
    #[arg(long, conflicts_with = "files")]
    base: Option<String>,

    /// Changed paths to plan from, skipping the diff entirely.
    ///
    /// For callers that already resolved the range: hk hands its own file list
    /// to a step and does not expose the refs it derived that list from, so
    /// re-diffing here would answer a different question than the gate asked.
    /// Passing the flag with no paths plans nothing, which is distinct from
    /// omitting it.
    #[arg(long, num_args = 0.., conflicts_with = "base")]
    files: Option<Vec<String>>,
}

/// The step output the test job reads, named for the key its `if:` guard
/// checks. Serialized field-for-field into `$GITHUB_OUTPUT`.
#[derive(serde::Serialize)]
struct TestPlan {
    filterset: String,
}

pub fn run(args: TestPlanArgs) -> Result<()> {
    let filterset = if args.release {
        union(RELEASE_ROOTS.iter().map(|root| format!("deps({root})")))
    } else {
        let changed = match args.files {
            Some(files) => files,
            None => {
                let base = match args.base {
                    Some(base) => base,
                    None => default_base()?,
                };
                changed_files(&base)?
            }
        };

        let metadata = crate::workspace::load()?;
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

        plan(&changed, &directories)
    };

    // Printed as well as published, so a manual run and the job log both show
    // the filterset.
    println!("{filterset}");
    crate::utils::github::set_outputs_if_available(&TestPlan { filterset })
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
    match shell::env("GITHUB_BASE_REF") {
        Ok(base) if !base.is_empty() => {
            shell::git(&["merge-base", &format!("origin/{base}"), "HEAD"])
        }
        _ => Ok("HEAD~1".to_owned()),
    }
}

fn changed_files(base: &str) -> Result<Vec<String>> {
    let out = shell::git(&["diff", "--name-only", base, "HEAD"])?;
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

    /// `.config/cargo.toml` rather than `.cargo/config.toml`: the latter is a
    /// symlink to the former, which git tracks as a symlink blob. Editing the
    /// cargo config therefore reports the real path as changed and leaves the
    /// link untouched, so the real path is what a plan has to widen on.
    #[test]
    fn a_toolchain_change_tests_everything() {
        // `.cargo/config.toml` is a symlink to the real file, so an edit lands
        // in the diff under `.config/` — the path the planner has to match.
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

    fn parse(args: &[&str]) -> TestPlanArgs {
        TestPlanArgs::try_parse_from(std::iter::once("test-plan").chain(args.iter().copied()))
            .expect("should parse")
    }

    /// `--files` with no paths means "nothing changed, plan nothing" — it must
    /// stay distinguishable from the flag being absent, or an empty hook file
    /// list would silently fall back to diffing and test the wrong range.
    #[test]
    fn an_empty_files_flag_is_not_the_same_as_no_files_flag() {
        assert_eq!(parse(&["--files"]).files, Some(vec![]));
        assert_eq!(parse(&[]).files, None);
    }

    #[test]
    fn files_collects_every_path() {
        assert_eq!(
            parse(&["--files", "crates/store/src/db.rs", "website/app/index.vue"]).files,
            Some(vec![
                "crates/store/src/db.rs".to_owned(),
                "website/app/index.vue".to_owned(),
            ])
        );
    }

    /// The two are different answers to "what changed"; taking both would mean
    /// silently preferring one.
    #[test]
    fn base_and_files_cannot_be_combined() {
        assert!(
            TestPlanArgs::try_parse_from(["test-plan", "--base", "HEAD~1", "--files", "a.rs"])
                .is_err()
        );
    }
}
