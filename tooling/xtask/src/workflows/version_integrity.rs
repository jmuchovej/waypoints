//! Checks what every pull request claims about versions, with
//! `cargo xtask version-integrity`.
//!
//! Advisory until a ruleset names it, and built for that day: a required check
//! that does not report leaves a pull request waiting forever, so there is no
//! path filter. Editing the title or description reruns it, since those are
//! what it reads.
//!
//! The job holds no credential, so it has no route to the planners' R2 cache,
//! and a pull-request-only workflow never saves to GitHub's from `main`. It
//! compiles `xtask` cold.

use super::{MiseEnv, WorkflowContext, helpers, runners, steps};
use gh_workflow::{Event, Job, Level, Permissions, PullRequest, PullRequestType, Workflow};

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let check = Job::new("version-integrity")
        .add_env(MiseEnv::select(&[MiseEnv::Xtask]))
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(runners::LINUX)
        // The diff is taken from the merge base, which a shallow clone does
        // not contain.
        .add_step(steps::checkout().add_with(("fetch-depth", 0)))
        .add_step(steps::setup_mise(false.into(), &context.mise_version))
        .add_step(steps::cargo_xtask("version-integrity", &[]).id("version-integrity"));

    helpers::named_workflow("check what a pull request claims about versions")
        .on(Event::default().pull_request(
            PullRequest::default()
                .add_branch("**")
                .add_type(PullRequestType::Opened)
                .add_type(PullRequestType::Synchronize)
                .add_type(PullRequestType::Reopened)
                .add_type(PullRequestType::Edited),
        ))
        .concurrency(helpers::one_run_per_ref("version-integrity"))
        .add_job("version-integrity", check)
}
