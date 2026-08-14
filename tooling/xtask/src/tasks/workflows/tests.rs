//! The test jobs.
//!
//! One job shape, two filtersets — see [`crate::tasks::test_plan`]. The plan is
//! computed at run time by `cargo xtask test-plan`, because the change-scoped
//! variant depends on a diff CI has and the generator does not.

use gh_workflow::{Expression, Job, Level, Permissions, Step};
use indoc::indoc;

use super::{
    bundle::NamedJob,
    utils::{runners::Platform, steps},
};

/// Which suite to run.
#[derive(Clone, Copy)]
pub(crate) enum Suite {
    /// Everything the shipped artifacts are built from. Gates a release.
    Release,
    /// Only what the change can break.
    Changed,
}

impl Suite {
    fn flag(self) -> &'static str {
        match self {
            Suite::Release => " --release",
            Suite::Changed => "",
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Suite::Release => "release",
            Suite::Changed => "affected",
        }
    }
}

/// Step id the plan is published under. Referenced by the run step's guard, so
/// a typo here silently disables testing — hence one constant, used twice.
const PLAN_STEP: &str = "plan";

/// Runs the suite on one platform.
///
/// Two steps rather than one: the plan decides *whether* there is anything to
/// test, and an empty plan skips the run instead of degrading into a
/// whole-workspace build. A website- or docs-only change plans nothing.
pub(crate) fn test(suite: Suite, platform: Platform) -> NamedJob {
    let plan = Step::new("Plan")
        .run(format!(
            indoc! {r#"
                filterset="$(cargo xtask test-plan{})"
                echo "filterset=${{filterset}}" >> "$GITHUB_OUTPUT"
                if [ -n "${{filterset}}" ]; then
                  echo "Testing: ${{filterset}}"
                else
                  echo "No Rust crates affected."
                fi
            "#},
            suite.flag()
        ))
        .id(PLAN_STEP);

    let run = Step::new(format!("Run {} tests", suite.describe()))
        .run(format!(
            "cargo nextest run --workspace --no-fail-fast -E \"${{{{ steps.{PLAN_STEP}.outputs.filterset }}}}\""
        ))
        .if_condition(Expression::new(format!(
            "steps.{PLAN_STEP}.outputs.filterset != ''"
        )));

    let job = Job::default()
        .name(format!("Test {platform}"))
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(platform.runner())
        // The change-scoped plan diffs against the merge base, which a shallow
        // clone does not contain.
        .add_step(steps::checkout().add_with(("fetch-depth", 0)))
        .add_step(steps::setup_mise())
        // After mise: the cache key fingerprints the resolved rustc version.
        .add_step(steps::rust_cache());

    // Tauri links WebKitGTK on Linux, so even a library test build needs it.
    let job = match platform {
        Platform::Linux => job.add_step(steps::install_linux_deps()),
        _ => job,
    };

    NamedJob {
        id: format!("test-{}", platform.slug()),
        job: job.add_step(plan).add_step(run),
    }
}
