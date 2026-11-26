// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Renders and checks the package manifests under `distribution/` whenever
//! they or the renderer change. Its own workflow, not a job in `Tests`: every
//! job there becomes a required check, and this one is not required.

use super::{WorkflowContext, helpers, jobs};
use gh_workflow::{Event, PullRequest, Push, Workflow};

/// Paths whose change can break a render or a channel's check.
const SOURCES: &[&str] = &[
    "distribution/**",
    "src-tauri/res/configs/flavors/**",
    "tooling/xtask/**",
    ".github/workflows/distribution.yaml",
];

pub(crate) fn workflow(context: &WorkflowContext) -> Workflow {
    let dist = jobs::dist(context);

    helpers::named_workflow("Distribution")
        .on(Event::default()
            .pull_request(
                SOURCES
                    .iter()
                    .fold(PullRequest::default().add_branch("**"), |pr, path| {
                        pr.add_path(*path)
                    }),
            )
            .push(
                SOURCES
                    .iter()
                    .fold(Push::default().add_branch("main"), |push, path| {
                        push.add_path(*path)
                    }),
            ))
        .concurrency(helpers::one_run_per_ref("distribution"))
        .add_job(dist.name, dist.job)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The job runs where `brew audit` can, and no ruleset requires it.
    #[test]
    fn the_check_runs_on_macos_and_is_not_required() {
        let context = WorkflowContext::fixture();
        let value = serde_json::to_value(workflow(&context)).unwrap();
        let job = &value["jobs"]["dist"];
        assert_eq!(job["runs-on"], "macos-26");
        assert!(
            job["steps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|step| step["run"] == "just dist::check-all")
        );

        let tfvars = super::super::render_tfvars(&context).unwrap().content;
        assert!(!tfvars.contains("Check the package manifests"), "{tfvars}");
    }
}
