// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Records a published tag in the release index.
//!
//! Runs on a tag's push, or by hand for a tag that already exists, so an
//! entry can never precede the tag it names: the ordering lives in the
//! trigger rather than in our control flow. The task classifies the channel
//! from the version, checks the tagged tree's manifest names the same,
//! prefetches the tree's hash, and writes the channel's file. `main` is
//! protected, so the entry lands through a pull request that merges itself once
//! its required checks pass. The job does not wait for it.
//!
//! The push trigger relies on the planners minting their tags with the
//! wp-botswain App token: a ref created with `GITHUB_TOKEN` raises no `push`
//! event, by GitHub's rule against workflows triggering each other. The App
//! token opens the pull request for the same reason, or its checks would never
//! run. The dispatch is for a tag that predates this, or a run that failed.

use super::WorkflowInput;
use super::{WorkflowContext, helpers, helpers::Environment, runners, steps, steps::Tools};
use crate::Channel;
use gh_workflow::{
    Environment as WorkflowEnvironment, Event, Expression, Job, Level, Permissions, Push, Step,
    StepOutput, Workflow, WorkflowDispatch,
};

/// Every release tag; the task tells the channels apart from the version.
const TAG_PATTERN: &str = "v*";

/// Becomes the merge commit's body, so it says what the merge commit is. GitHub
/// wraps that body at 72 columns, so this stays under them.
pub(super) const BODY: &str =
    "Written by `cargo xtask release-index`; merges when its checks pass.";

/// Passed only when the dispatch asks for it: compute and print the entry,
/// commit nothing. Absent on a push, where `inputs` is empty.
const DRY_RUN_ON_REQUEST: &str = "${{ inputs.dry_run == true && '--dry-run' || '' }}";

/// The repository is private: the prefetch reads it through the github
/// fetcher, which takes its credential from here.
const ACCESS_TOKEN: &str = "access-tokens = github.com=${{ secrets.GITHUB_TOKEN }}";

pub fn workflow(_context: &WorkflowContext) -> Workflow {
    let tag = WorkflowInput::string("tag", None).description("An existing tag to record");
    let dry_run = WorkflowInput::bool("dry_run", Some(true))
        .description("Compute and print the entry without committing it");

    // Inside the devenv shell, as `check-generated` and `devenv-update` run:
    // the task drives `nix`, `git`, and `cargo`, and the shell is where they
    // come from at the versions the tree pins. The tag reaches the command
    // through the environment — `github.ref_name` is free text as far as the
    // shell is concerned.
    let record = Step::new("cargo xtask release-index")
        .run(format!(
            "devenv shell -- cargo xtask release-index --tag \"$TAG\" {DRY_RUN_ON_REQUEST}"
        ))
        .shell("bash")
        .id("release-index")
        .add_env(("TAG", helpers::triggering_tag()))
        .add_env((
            "SECRETSPEC_REASON",
            "release-index records a published tag in nix/releases; no secret values are read",
        ));

    // The task publishes these only when it wrote, so a dry run opens nothing.
    let wrote = || {
        Expression::new(format!(
            "{} != ''",
            StepOutput::new_trusted(&record, "title").expr()
        ))
    };
    let credentials = steps::botswain_credentials(Tools::Devenv).if_condition(wrote());
    let app_token = steps::app_token(&[("contents", "write"), ("pull-requests", "write")])
        .if_condition(wrote());
    let token = StepOutput::new_trusted(&app_token, "token");
    let pull_request =
        steps::land_write(&record, &token, "index-pull-request", BODY).if_condition(wrote());
    let auto_merge = steps::auto_merge(
        &StepOutput::new_trusted(&pull_request, "pull-request-number"),
        &token,
        Tools::Devenv,
    );

    // The devenv shell leaves `CARGO_HOME` unset, so cargo keeps its registry
    // at its default; the snapshot has to land there.
    let registry = steps::cargo_registry("${HOME}/.cargo");

    let job = Job::new("record the release in the index")
        // Only what `checkout` needs: the entry lands with the App token, which
        // carries its own scope.
        .permissions(Permissions::default().contents(Level::Read))
        // Compiles through the CI cache and authenticates as the App, so the
        // vault credential comes from the automation environment.
        .environment(WorkflowEnvironment::from(Environment::Ci))
        .runs_on(runners::LINUX)
        // On the branch the entry lands on, whatever ref triggered this: the
        // pull-request step commits against what is checked out. The tag
        // resolves to its commit here; nothing walks history.
        .add_step(
            steps::checkout()
                .add_with(("ref", "main"))
                .add_with(("fetch-tags", true)),
        )
        // Entering the shell realises the whole environment, Android SDK and
        // all, which a stock image cannot hold.
        .add_step(steps::free_disk_space(true))
        .add_step(steps::install_nix().add_with(("extra_nix_config", ACCESS_TOKEN)))
        // A Nix binary cache, not an Actions cache: nothing a workflow run
        // could have written into it, so no poisoning surface.
        .add_step(steps::cachix("devenv"))
        .add_step(steps::nix_profile_install(&["devenv"]))
        // The task is a `cargo xtask` compile before it is anything else; the
        // build jobs' registry snapshot and compiler cache pay for most of it.
        .add_step(steps::cache_restore(&registry, Tools::Devenv))
        .add_step(steps::sccache_start(Tools::Devenv))
        .add_step(steps::with_sccache(record))
        .add_step(steps::sccache_stats(Tools::Devenv))
        .add_step(steps::cache_save(&registry, Tools::Devenv))
        // Minted after the compile, which is most of this job's time.
        .add_step(credentials)
        .add_step(app_token)
        .add_step(pull_request)
        .add_step(auto_merge);

    helpers::named_workflow("record a published release in the index")
        .on(Event::default()
            .push(Push::default().add_tag(TAG_PATTERN))
            .workflow_dispatch(WorkflowDispatch::default().inputs([
                (tag.name.into(), tag.input()),
                (dry_run.name.into(), dry_run.input()),
            ])))
        // Two tags on one channel would race on its file's blob sha; queued,
        // the second write sees the first.
        .concurrency(helpers::serialize("release-index"))
        .add_job("release-index", job)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pushed ref name never lands in the command line, where it would
    /// be interpreted by the shell.
    #[test]
    fn the_tag_travels_through_the_environment() {
        let workflow = workflow(&WorkflowContext::fixture());
        let value = serde_json::to_value(&workflow).unwrap();
        let step = &value["jobs"]["release-index"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["id"] == "release-index")
            .expect("the recording step")
            .clone();

        let run = step["run"].as_str().unwrap();
        assert!(run.starts_with("devenv shell -- cargo xtask"), "{run}");
        assert!(run.contains("--tag \"$TAG\""), "{run}");
        assert!(!run.contains("github.ref_name"), "{run}");
        assert!(run.contains("--dry-run"), "{run}");
        assert!(
            step["env"]["TAG"]
                .as_str()
                .unwrap()
                .contains("github.ref_name"),
            "{}",
            step["env"]
        );
    }

    fn job() -> serde_json::Value {
        let value = serde_json::to_value(workflow(&WorkflowContext::fixture())).unwrap();
        value["jobs"]["release-index"].clone()
    }

    /// The pull-request step commits against what is checked out, and a tag push
    /// would otherwise check out the tag: the entry must land on `main`, and
    /// never in the tree it describes.
    #[test]
    fn the_job_is_on_main_with_the_tags_at_hand() {
        let job = job();
        let checkout = job["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["id"] == "checkout")
            .unwrap();
        assert_eq!(checkout["with"]["ref"], "main");
        assert_eq!(checkout["with"]["fetch-tags"], "true");
    }

    /// `GITHUB_TOKEN` raises no `pull_request` event, so an entry it opened would
    /// wait forever on checks that never run.
    #[test]
    fn the_entry_lands_with_the_app_token_and_not_the_job_s() {
        let job = job();
        assert_eq!(job["permissions"]["contents"], "read");

        let steps = job["steps"].as_array().unwrap();
        let pull_request = steps
            .iter()
            .find(|step| step["id"] == "index-pull-request")
            .unwrap();
        assert_eq!(
            pull_request["with"]["token"],
            "${{ steps.app-token.outputs.token }}"
        );
        assert_eq!(pull_request["with"]["sign-commits"], "true");
    }

    /// A dry run publishes no landing, so everything downstream stands down.
    #[test]
    fn a_dry_run_opens_nothing() {
        let job = job();
        let steps = job["steps"].as_array().unwrap();
        let wrote = "steps.release-index.outputs.title != ''";

        for id in ["app-token", "index-pull-request"] {
            let step = steps.iter().find(|step| step["id"] == id).unwrap();
            assert_eq!(step["if"], wrote, "{id}");
        }
        let credentials = steps
            .iter()
            .find(|step| step["name"] == "Resolve the botswain secrets")
            .unwrap();
        assert_eq!(credentials["if"], wrote);

        let auto_merge = steps
            .iter()
            .find(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains("release-land auto-merge"))
            })
            .unwrap();
        assert_eq!(
            auto_merge["if"],
            "steps.index-pull-request.outputs.pull-request-number != ''"
        );
    }

    /// This job's tools live in the devenv shell, which refuses to enter without
    /// a stated reason.
    #[test]
    fn the_landing_runs_inside_the_shell() {
        let job = job();
        let auto_merge = job["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains("release-land auto-merge"))
            })
            .unwrap()
            .clone();
        assert!(
            auto_merge["run"]
                .as_str()
                .unwrap()
                .starts_with("devenv shell -- cargo xtask"),
            "{auto_merge}"
        );
        assert!(auto_merge["env"].get("SECRETSPEC_REASON").is_some());
    }
}
