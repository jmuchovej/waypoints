//! Tests on every pull request and every push to `main`, scoped to the crates
//! `cargo xtask test-plan` resolves from the diff.

use gh_workflow::{Event, Level, Permissions, PullRequest, Push, Workflow};

use super::{
    Platform, TestSuite, WorkflowContext,
    helpers::{FORCE_NODE24, one_run_per_ref},
    jobs,
};

pub(crate) fn workflow(_: &WorkflowContext) -> Workflow {
    let job = jobs::test(TestSuite::Changed, Platform::Linux);

    Workflow::new("Tests")
        .on(Event::default()
            .pull_request(PullRequest::default().add_branch("**"))
            .push(Push::default().add_branch("main")))
        .permissions(Permissions::default().contents(Level::Read))
        .concurrency(one_run_per_ref("tests"))
        .add_env(FORCE_NODE24)
        .add_job(job.name, job.job)
}
