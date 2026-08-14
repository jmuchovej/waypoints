//! Tests on every pull request and every push to `main`.
//!
//! Scoped to what changed: `cargo xtask test-plan` resolves the diff to the
//! affected crates and their in-tree dependants, and widens to the whole
//! workspace whenever a change cannot be attributed to a subset.

use gh_workflow::{Event, Level, Permissions, PullRequest, Push, Workflow};

use super::{
    WorkflowContext,
    tests::{self, Suite},
    utils::{Platform, vars},
};

pub(crate) fn workflow(_: &WorkflowContext) -> Workflow {
    let job = tests::test(Suite::Changed, Platform::Linux);

    Workflow::new("Tests")
        .on(Event::default()
            .pull_request(PullRequest::default().add_branch("**"))
            .push(Push::default().add_branch("main")))
        .permissions(Permissions::default().contents(Level::Read))
        .concurrency(vars::one_run_per_ref("tests"))
        .add_env(vars::FORCE_NODE24)
        .add_job(job.id, job.job)
}
