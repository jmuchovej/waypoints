// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Checks what every pull request claims about versions, with
//! `cargo xtask check version-integrity`.
//!
//! Advisory until a ruleset names it, and built for that day: a required check
//! that does not report leaves a pull request waiting forever, so there is no
//! path filter. Editing the title or description reruns it, since those are
//! what it reads.
//!
//! The jobs hold no credential, so they have no route to the planners' R2
//! cache, and a pull-request-only workflow never saves to GitHub's from
//! `main`. They compile `xtask` cold.
//!
//! Beside the claims check runs the API floor's measurement, `semver`:
//! `cargo-semver-checks`, as a library of `xtask`, over each library group
//! the pull request touches, against the group's last tag, under the stable
//! flavor (library-minting design D5). One measurement is the contract, so
//! it runs on Linux alone;
//! a group with no tag yet is skipped by name, so it is inert until the
//! first cut mints a baseline (D8).

use super::{MiseEnv, WorkflowContext, actions, actions::MiseSetup, helpers, runners, steps};
use gh_workflow::{Event, Job, Level, Permissions, PullRequest, PullRequestType, Workflow};

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let check = Job::new("version-integrity")
        .add_env(MiseEnv::select(&[MiseEnv::Xtask]))
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(runners::LINUX)
        // The diff is taken from the merge base, which a shallow clone does
        // not contain.
        .add_step(steps::checkout().add_with(("fetch-depth", 0)))
        .add_step(actions::setup_mise(
            MiseSetup::default(),
            &context.mise_version,
        ))
        .add_step(steps::cargo_xtask("check version-integrity", &[]).id("version-integrity"));

    let semver = Job::new("semver")
        .add_env(MiseEnv::select(&[MiseEnv::Xtask]))
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(runners::LINUX)
        // The baseline is a tag checked out as a worktree, so the tags and
        // the history behind them have to be here.
        .add_step(
            steps::checkout()
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true)),
        )
        .add_step(actions::setup_mise(
            MiseSetup::default(),
            &context.mise_version,
        ))
        .add_step(steps::cargo_xtask("check semver", &[]).id("semver-check"));

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
        .add_job("semver", semver)
}
