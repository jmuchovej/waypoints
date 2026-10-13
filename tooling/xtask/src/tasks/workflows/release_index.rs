//! Records a published tag in the release index.
//!
//! Runs on a tag's push, or by hand for a tag that already exists, so an
//! entry can never precede the tag it names: the ordering lives in the
//! trigger rather than in our control flow. The task classifies the channel
//! from the version, checks the tagged tree declares the same, prefetches
//! the tree's hash, and commits the channel's file to `main` through the
//! Contents API, which needs only `GITHUB_TOKEN`.
//!
//! A tag minted with `GITHUB_TOKEN` raises no `push` event — GitHub's rule
//! against workflows triggering each other — and today both planners mint
//! theirs that way. Until they mint with the App token, those tags are
//! recorded through the dispatch.

use super::{Channel, WorkflowContext, WorkflowInput, helpers, runners, steps};
use gh_workflow::{Event, Job, Level, Permissions, Push, Step, Workflow, WorkflowDispatch};

/// Every release tag; the task tells the channels apart from the version.
const TAG_PATTERN: &str = "v*";

/// Passed only when the dispatch asks for it: compute and print the entry,
/// commit nothing. Absent on a push, where `inputs` is empty.
const DRY_RUN_ON_REQUEST: &str = "${{ inputs.dry_run == true && '--dry-run' || '' }}";

/// The repository is private: the prefetch reads it through the github
/// fetcher, which takes its credential from here.
const ACCESS_TOKEN: &str = "access-tokens = github.com=${{ secrets.GITHUB_TOKEN }}";

pub fn workflow(_context: &WorkflowContext) -> Workflow {
    let tag = WorkflowInput::string("tag", None).description("An existing tag to record");
    let dry_run = WorkflowInput::bool("dry_run", Some(false))
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
        .add_env(("TAG", Channel::tag(None)))
        .add_env(("GH_TOKEN", helpers::GITHUB_TOKEN))
        .add_env((
            "SECRETSPEC_REASON",
            "release-index records a published tag in nix/releases; no secret values are read",
        ));

    let job = Job::new("record the release in the index")
        // Write: the entry is committed to `main`.
        .permissions(Permissions::default().contents(Level::Write))
        .runs_on(runners::LINUX)
        // The tag resolves to its commit here; nothing walks history.
        .add_step(steps::checkout().add_with(("fetch-tags", true)))
        // Entering the shell realises the whole environment, Android SDK and
        // all, which a stock image cannot hold.
        .add_step(steps::free_disk_space(true))
        .add_step(steps::install_nix().add_with(("extra_nix_config", ACCESS_TOKEN)))
        // A Nix binary cache, not an Actions cache: nothing a workflow run
        // could have written into it, so no poisoning surface.
        .add_step(steps::cachix("devenv"))
        .add_step(steps::nix_profile_install(&["devenv"]))
        .add_step(record);

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
}
