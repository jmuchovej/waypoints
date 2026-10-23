//! Prints the nextest filterset a CI run should use. See
//! [`xtask::planners::test`].

use anyhow::Result;
use clap::Parser;
use xtask::{planners::test as test_plan, tools::github, workspace};

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
}

/// The step output the test job reads, named for the key its `if:` guard
/// checks. Serialized field-for-field into `$GITHUB_OUTPUT`.
#[derive(serde::Serialize)]
struct TestPlan {
    filterset: String,
}

pub fn run(args: Options) -> Result<()> {
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

        let metadata = workspace::load()?;
        test_plan::plan(&changed, &test_plan::package_directories(&metadata))
    };

    // Printed as well as published, so a manual run and the job log both show
    // the filterset.
    println!("{filterset}");
    github::set_outputs_if_available(&TestPlan { filterset })
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
}
