// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tests on every pull request and every push to `main`, scoped to the crates
//! `cargo xtask test-plan` resolves from the diff.

use super::TestSuite;
use super::{WorkflowContext, helpers, jobs};
use crate::Platform;
use gh_workflow::{Event, PullRequest, Push, Workflow};

pub(crate) fn workflow(context: &WorkflowContext) -> Workflow {
    let job = jobs::test(TestSuite::Changed, Platform::Linux, context);
    let audit = jobs::zizmor(context);
    let license = jobs::license();

    helpers::named_workflow("Tests")
        .on(Event::default()
            .pull_request(PullRequest::default().add_branch("**"))
            .push(Push::default().add_branch("main")))
        .concurrency(helpers::one_run_per_ref("tests"))
        .add_job(job.name, job.job)
        .add_job(audit.name, audit.job)
        .add_job(license.name, license.job)
}
