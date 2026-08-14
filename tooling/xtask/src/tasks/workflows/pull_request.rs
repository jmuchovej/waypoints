//! Tests on every pull request and every push to `main`, scoped to the crates
//! `cargo xtask test-plan` resolves from the diff.

use super::{Platform, TestSuite, WorkflowContext, helpers, jobs};
use gh_workflow::{Event, PullRequest, Push, Workflow};

pub(crate) fn workflow(_: &WorkflowContext) -> Workflow {
    let job = jobs::test(TestSuite::Changed, Platform::Linux);

    helpers::named_workflow("Tests")
        .on(Event::default()
            .pull_request(PullRequest::default().add_branch("**"))
            .push(Push::default().add_branch("main")))
        .concurrency(helpers::one_run_per_ref("tests"))
        .add_job(job.name, job.job)
}
