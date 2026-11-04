//! Prints the nextest filterset a CI run should use. See
//! [`xtask::planners::test`].

use anyhow::{Context as _, Result};
use clap::Parser;
use std::fs;
use xtask::{
    planners::test as test_plan, releases::version::APP_MANIFEST, tools::github, workspace,
};

#[derive(Parser)]
pub struct Options {
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

    /// Print only the flavor the tree ships as, for the hooks' `--features`.
    #[arg(long, conflicts_with_all = ["release", "base", "files"])]
    flavor: bool,
}

/// The step outputs the test job reads: `filterset` for its `if:` guard, and
/// `flavor` for the suite's `--features`. Serialized field-for-field into
/// `$GITHUB_OUTPUT`.
#[derive(serde::Serialize)]
struct TestPlan {
    filterset: String,
    flavor: String,
}

pub fn run(args: Options) -> Result<()> {
    let metadata = workspace::load()?;
    let manifest = metadata.workspace_root.join(APP_MANIFEST);
    let flavor = test_plan::flavor(
        &fs::read_to_string(&manifest).with_context(|| format!("Failed to read {manifest}"))?,
    )?;
    if args.flavor {
        println!("{flavor}");
        return Ok(());
    }

    let filterset = if args.release {
        test_plan::release_plan()
    } else {
        let changed = match args.files {
            Some(files) => files,
            None => {
                let base = match args.base {
                    Some(base) => base,
                    None => test_plan::default_base()?,
                };
                test_plan::changed_files(&base)?
            }
        };

        test_plan::plan(&changed, &test_plan::package_directories(&metadata))
    };

    // Printed as well as published, so a manual run and the job log both show
    // the plan. Only the filterset goes to stdout: hk reads it from there.
    println!("{filterset}");
    eprintln!("flavor: {flavor}");
    github::set_outputs_if_available(&TestPlan { filterset, flavor })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Options {
        Options::try_parse_from(std::iter::once("test-plan").chain(args.iter().copied()))
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
            Options::try_parse_from(["test-plan", "--base", "HEAD~1", "--files", "a.rs"]).is_err()
        );
    }

    /// The hooks read `--flavor`'s stdout as one feature name, so it plans
    /// nothing else.
    #[test]
    fn flavor_stands_alone() {
        assert!(parse(&["--flavor"]).flavor);
        for other in [
            &["--release"][..],
            &["--base", "HEAD~1"],
            &["--files", "a.rs"],
        ] {
            let args = std::iter::once("test-plan")
                .chain(other.iter().copied())
                .chain(["--flavor"]);
            assert!(Options::try_parse_from(args).is_err(), "{other:?}");
        }
    }
}
