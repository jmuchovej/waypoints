// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Mints the refs a release cycle moves through.
//!
//! Every transition is a deliberate decision rather than a schedule, so this
//! is dispatch-only. One branch exists per minor version: `cut` opens it as a
//! preview, `advance` takes the next candidate on it, `promote` makes it
//! stable in place, and `patch` fixes it afterwards. Which of those a run
//! performs is the `transition` input; the planner derives everything else
//! from the manifest version and the tags already published.
//!
//! Nightlies are minted by [`super::release`]'s `prepare` job instead — their
//! version is a timestamp, not a claim about content.
//!
//! Refs are minted with the wp-botswain App token rather than `GITHUB_TOKEN`:
//! a tag pushed with the latter raises no `push` event, so neither the
//! channel's release workflow nor the release index would run on it.
//!
//! `main` and the series branches are protected, so the version write lands
//! through a pull request gated like any other, and the tag goes on its merge
//! commit. The job waits for that merge, which can outlast an App token, so it
//! looks under `GITHUB_TOKEN` and mints a fresh token for the tag.

use super::{MiseEnv, WorkflowInput};
use super::{WorkflowContext, helpers, helpers::Environment, runners, steps, steps::Tools};
use gh_workflow::{
    Environment as GhEnvironment, Event, Expression, Job, Level, Permissions, StepOutput, Workflow,
    WorkflowDispatch,
};

/// Becomes the merge commit's body, so it says what the merge commit is. GitHub
/// wraps that body at 72 columns, so this stays under them.
pub(super) const BODY: &str = "Written by `cargo xtask release-plan`; merges when its checks pass.";

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let transition = WorkflowInput::choices(
        "transition",
        crate::releases::version::transitions(),
        None::<String>,
    )
    .description("Which release-cycle transition to perform");

    let app_token = steps::app_token(&[("contents", "write"), ("pull-requests", "write")]);
    let token = StepOutput::new_trusted(&app_token, "token");
    let registry = steps::cargo_registry(steps::CARGO_HOME);

    let planner = steps::cargo_xtask("release-plan", &[("${{ inputs.transition }}", None)])
        .id("release-plan")
        .add_env(("GH_TOKEN", token.to_string()));
    let output = |name| StepOutput::new_trusted(&planner, name);
    let next_main = output("next_main_version");

    let series = steps::land_write(&planner, &token, "series-pull-request", BODY);
    let series_number = StepOutput::new_trusted(&series, "pull-request-number");

    let bump = steps::bump_main(&next_main);
    let main = steps::land_write(&bump, &token, "main-pull-request", BODY)
        .if_condition(Expression::new(format!("{} != ''", next_main.expr())));
    let main_number = StepOutput::new_trusted(&main, "pull-request-number");

    let wait = steps::wait_for_merge(&series_number);
    let merge_commit = StepOutput::new_trusted(&wait, "merge_commit");

    let tag_token = steps::app_token(&[("contents", "write")])
        .name("Mint a wp-botswain token for the tag")
        .id("tag-token");
    let tag = steps::tag_merge_commit(
        &output("tag"),
        &merge_commit,
        &StepOutput::new_trusted(&tag_token, "token"),
    );
    let auto_merge_series = steps::auto_merge(&series_number, &token, Tools::Mise);
    let auto_merge_main = steps::auto_merge(&main_number, &token, Tools::Mise);

    let plan = Job::new("plan the transition")
        // A `cargo xtask` run and nothing more.
        .add_env(MiseEnv::select(&[MiseEnv::Xtask]))
        // Only what `checkout` and the wait need: every write goes through the
        // App token, which carries its own scope.
        .permissions(
            Permissions::default()
                .contents(Level::Read)
                .pull_requests(Level::Read),
        )
        // Authenticates as the App and compiles through the CI cache, so the
        // vault credential comes from the automation environment.
        .environment(GhEnvironment::from(Environment::Ci))
        .runs_on(runners::LINUX)
        // Tags and the history between them: a cut walks from the last stable
        // release to HEAD with a pathspec that drops the index directory, and
        // in a shallow clone the tip is a root commit that appears to touch
        // every path — see the nightly planner.
        .add_step(
            steps::checkout()
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true)),
        )
        .add_step(steps::setup_mise(false.into(), &context.mise_version))
        // As with the nightly planner: the cost here is compiling xtask, and
        // the build jobs' registry snapshot and compiler cache answer most of
        // it.
        .add_step(steps::cache_restore(&registry, Tools::Mise))
        .add_step(steps::sccache_start(Tools::Mise))
        .add_step(steps::botswain_credentials(Tools::Mise))
        .add_step(app_token)
        .add_step(steps::with_sccache(planner))
        // Before the pull requests: the wait below is long, and this stops the
        // server holding the cache credential.
        .add_step(steps::sccache_stats(Tools::Mise))
        .add_step(steps::cache_save(&registry, Tools::Mise))
        .add_step(series)
        .add_step(auto_merge_series)
        // A cut's two pull requests open together and land independently. If the
        // series' checks fail, `main` has still moved on, and the series recovers
        // with `advance`; no transition could move `main` later.
        .add_step(bump)
        .add_step(main)
        .add_step(auto_merge_main)
        .add_step(wait)
        .add_step(tag_token)
        .add_step(tag);

    helpers::named_workflow("mint the refs for a release-cycle transition")
        .on(Event::default().workflow_dispatch(
            WorkflowDispatch::default().inputs([(transition.name.into(), transition.input())]),
        ))
        // A half-applied transition leaves the branch and its tag disagreeing.
        .concurrency(helpers::serialize("release-plan"))
        .add_job("release-plan", plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn steps() -> Vec<Value> {
        let value = serde_json::to_value(workflow(&WorkflowContext::fixture())).unwrap();
        value["jobs"]["release-plan"]["steps"]
            .as_array()
            .unwrap()
            .clone()
    }

    fn position(steps: &[Value], id: &str) -> usize {
        steps
            .iter()
            .position(|step| step["id"] == id)
            .unwrap_or_else(|| panic!("no step `{id}`"))
    }

    fn running(steps: &[Value], command: &str) -> usize {
        steps
            .iter()
            .position(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains(command))
            })
            .unwrap_or_else(|| panic!("no step runs `{command}`"))
    }

    /// A tag made before the merge names a commit the planners' first-parent
    /// walk never reaches.
    #[test]
    fn the_tag_waits_for_the_merge_commit() {
        let steps = steps();
        let tag = running(&steps, "release-land tag");

        assert!(position(&steps, "wait-for-merge") < tag);
        assert_eq!(
            steps[tag]["env"]["COMMIT"],
            "${{ steps.wait-for-merge.outputs.merge_commit }}"
        );
        assert_eq!(
            steps[tag]["env"]["TAG"],
            "${{ steps.release-plan.outputs.tag }}"
        );
    }

    /// An App token lasts an hour, and the required checks can take most of it.
    /// A token that expired mid-wait would land the write and lose the tag.
    #[test]
    fn the_tag_gets_a_token_minted_after_the_wait() {
        let steps = steps();
        let wait = position(&steps, "wait-for-merge");
        assert_eq!(steps[wait]["env"]["GH_TOKEN"], helpers::GITHUB_TOKEN);

        assert!(wait < position(&steps, "tag-token"));
        assert_eq!(
            steps[running(&steps, "release-land tag")]["env"]["GH_TOKEN"],
            "${{ steps.tag-token.outputs.token }}"
        );
    }

    /// The tag ref is the only thing this token writes.
    #[test]
    fn the_token_for_the_tag_can_only_write_contents() {
        let steps = steps();
        let token = &steps[position(&steps, "tag-token")];
        let scopes: Vec<&String> = token["with"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| key.starts_with("permission-"))
            .collect();
        assert_eq!(scopes, ["permission-contents"]);
    }

    /// In this order, so a failed series leaves `main` moved and the series
    /// recoverable with `advance`, rather than `main` stranded on a version no
    /// transition moves.
    #[test]
    fn a_cut_opens_both_pull_requests_before_the_wait() {
        let steps = steps();
        let order = [
            position(&steps, "release-plan"),
            position(&steps, "series-pull-request"),
            position(&steps, "bump-main"),
            position(&steps, "main-pull-request"),
            position(&steps, "wait-for-merge"),
        ];
        assert!(order.is_sorted(), "{order:?}");
    }

    #[test]
    fn only_a_cut_bumps_main() {
        let steps = steps();
        for id in ["bump-main", "main-pull-request"] {
            assert_eq!(
                steps[position(&steps, id)]["if"],
                "steps.release-plan.outputs.next_main_version != ''",
                "{id}"
            );
        }
    }

    /// Each pull request is landed by the auto-merge step reading its own number.
    #[test]
    fn each_pull_request_merges_itself() {
        let steps = steps();
        let numbers: Vec<&str> = steps
            .iter()
            .filter(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains("release-land auto-merge"))
            })
            .map(|step| step["env"]["PULL_REQUEST"].as_str().unwrap())
            .collect();
        assert_eq!(
            numbers,
            [
                "${{ steps.series-pull-request.outputs.pull-request-number }}",
                "${{ steps.main-pull-request.outputs.pull-request-number }}",
            ]
        );
    }

    /// The compiler cache's credential should not sit in a server for the length
    /// of the wait.
    #[test]
    fn the_cache_server_stops_before_the_wait() {
        let steps = steps();
        assert!(position(&steps, "sccache-stats") < position(&steps, "series-pull-request"));
    }
}
