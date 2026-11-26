// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The release: one workflow for every channel, started by the push of a
//! release tag — the one `bump` minted for it minutes before — by a dispatch
//! naming an existing tag, or by a call. It plans nothing and mints nothing:
//! the tag carries the channel, the version, the flavor, and the store build
//! codes, which the builder's `describe` job reads from it, and a rehearsal
//! dispatched without a tag describes the day's nightly from the commit it
//! runs on. The builder tests and builds the tree; this workflow drafts the
//! record, publishes in order of retractability with the manifest the last
//! write to the store, and records the index entry after it, on a real run
//! only, so a withdrawn tag is never recorded.
//!
//! A run that fails or falls short before a store holds the build withdraws:
//! the draft, the objects it uploaded, and — when the run was the tag's own
//! push — the tag and the bump that minted it, the write reverted through a
//! pull request the bot opens and waits on, so the files and the tags
//! disagree for no longer than a revert takes to land. A dispatched
//! re-publication leaves the tag and its write, since that release may
//! already be out.
//!
//! One queued concurrency group, shared with `bump`: one release cycle runs
//! at a time, so a scheduled nightly waits out a release and cannot mint from
//! a `main` that run is about to revert. The dispatch's inputs are read where
//! the event is a dispatch; a caller's inputs are read on the same terms, so
//! the first caller settles how its own event reads them.

use super::{ReleaseRun, WorkflowContext, WorkflowInput, helpers, jobs};
use gh_workflow::{
    Event, Expression, FluentBuilder, NamedJob, Push, Workflow, WorkflowCall, WorkflowDispatch,
};

/// Every release tag; the task tells the channels apart from the version. A
/// library's `<group>-v…` tag matches neither this nor a channel's pattern,
/// so a group bump starts no release.
pub(super) const TAG_PATTERN: &str = "v*";

/// Becomes the record's merge commit body, so it says what the merge commit
/// is.
pub(super) fn body() -> String {
    jobs::record_body()
}

pub fn workflow(context: &WorkflowContext) -> Workflow {
    // Defaulted to empty, not unset: an input with no default is `required`,
    // and a dry run may leave it blank to describe the commit it runs on.
    let tag = WorkflowInput::string("tag", Some(String::new())).description(
        "The existing tag to publish; a dry run left blank describes the day's nightly",
    );
    // Every dispatch rehearses unless it says otherwise, so a forgotten flag
    // never re-publishes.
    let dry_run = WorkflowInput::bool("dry_run", Some(true)).description(
        "Rehearse: build as asked, write only to the run's artifacts, publish nothing",
    );
    let build = WorkflowInput::bool("build", Some(true))
        .description("Run the test and build stages; off, the run only guards and reports");
    let platforms =
        WorkflowInput::choices("platforms", helpers::dispatch_platforms(), "All".into())
            .description("Platforms to build");

    let guard = jobs::guard(&tag, &platforms, &dry_run, &build);
    // The tree the builder works on: the pushed tag, the dispatched tag, or
    // the commit a rehearsal without one runs on. `platforms` is `All` on a
    // push, where the inputs are empty.
    let builder = jobs::call_builder(
        &helpers::release_ref(),
        &format!("${{{{ {} || 'All' }}}}", platforms.expr()),
        &guard,
        Expression::new(format!(
            "github.event_name != 'workflow_dispatch' || {}",
            build.expr()
        )),
    );
    let run = ReleaseRun::new(&guard, &builder);

    let create = jobs::create_release(&run, context);
    let publish = jobs::publish(&run).map(|job| job.add_needs(create.name.clone()));
    let record = jobs::record(&run, context);
    // Cleans up a run that failed or fell short, on the same `needs` as the
    // publish it mirrors, and on the publish itself for its point of no
    // return.
    let withdraw = jobs::withdraw_release(&run).map(|job| {
        job.add_needs(create.name.clone())
            .add_needs(publish.name.clone())
    });

    let dispatch = WorkflowDispatch::default().inputs([
        (tag.name.into(), tag.input()),
        (dry_run.name.into(), dry_run.input()),
        (build.name.into(), build.input()),
        (platforms.name.into(), platforms.input()),
    ]);
    let call = WorkflowCall::default()
        .add_input(tag.name, tag.call_input())
        .add_input(dry_run.name, dry_run.call_input())
        .add_input(build.name, build.call_input())
        .add_input(platforms.name, platforms.call_input());

    let guard: NamedJob = guard.into();
    helpers::named_workflow("Release")
        .on(Event::default()
            .push(Push::default().add_tag(TAG_PATTERN))
            .workflow_dispatch(dispatch)
            .workflow_call(call))
        .concurrency(helpers::serialize(helpers::RELEASE_CYCLE))
        .add_job(guard.name, guard.job)
        .map(|workflow| {
            [builder, create, publish, record, withdraw]
                .into_iter()
                .fold(workflow, |workflow, named| {
                    workflow.add_job(named.name, named.job)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Platform;
    use serde_json::Value;
    use strum::IntoEnumIterator as _;

    fn rendered() -> Value {
        serde_json::to_value(workflow(&WorkflowContext::fixture())).expect("workflow serializes")
    }

    fn needs(job: &Value) -> Vec<&str> {
        job["needs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|need| need.as_str().unwrap())
            .collect()
    }

    /// Scenario _A pushed tag starts its release_: the one push trigger is
    /// every release tag, the dispatch names an existing tag and rehearses
    /// by default, and a call takes the same four.
    #[test]
    fn a_pushed_tag_starts_its_release() {
        let value = rendered();
        let on = value["on"].as_object().expect("triggers");
        let mut triggers: Vec<&str> = on.keys().map(String::as_str).collect();
        triggers.sort_unstable();
        assert_eq!(triggers, ["push", "workflow_call", "workflow_dispatch"]);
        assert_eq!(on["push"]["tags"], serde_json::json!([TAG_PATTERN]));
        assert!(on["push"].get("branches").is_none(), "{}", on["push"]);

        let inputs = on["workflow_dispatch"]["inputs"]
            .as_object()
            .expect("inputs");
        let mut names: Vec<&str> = inputs.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, ["build", "dry_run", "platforms", "tag"]);
        let default = |name: &str| inputs[name]["default"].to_string().replace('"', "");
        assert_eq!(default("dry_run"), "true");
        assert_eq!(default("build"), "true");
        assert_eq!(default("platforms"), "All");
        assert_eq!(default("tag"), "");

        let called = on["workflow_call"]["inputs"]
            .as_object()
            .expect("call inputs");
        let mut names: Vec<&str> = called.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, ["build", "dry_run", "platforms", "tag"]);
        assert_eq!(called["platforms"]["type"], "string");

        assert_eq!(value["concurrency"]["group"], helpers::RELEASE_CYCLE);
        assert_eq!(value["concurrency"]["cancel-in-progress"], false);
    }

    /// Scenario _A rehearsal without a tag describes the day's nightly_: the
    /// builder is called on the pushed tag, the dispatched tag, or the commit
    /// the run is on, with the platforms asked for, and only when the
    /// dispatch did not turn the build off.
    #[test]
    fn the_builder_is_called_on_the_release_s_ref() {
        let value = rendered();
        let build = &value["jobs"]["build"];
        assert_eq!(
            build["uses"],
            "$/.github/workflows/build-distributables.yaml"
        );
        assert_eq!(build["with"]["ref"], helpers::release_ref());
        assert!(
            helpers::release_ref().contains("github.ref_name")
                && helpers::release_ref().contains("inputs.tag")
                && helpers::release_ref().ends_with("github.sha }}"),
            "{}",
            helpers::release_ref()
        );
        assert_eq!(
            build["with"]["platforms"],
            "${{ inputs.platforms || 'All' }}"
        );
        assert_eq!(
            build["if"],
            "github.event_name != 'workflow_dispatch' || inputs.build"
        );
        assert_eq!(needs(build), ["guard"]);
        assert_eq!(build["secrets"], "inherit");
        assert!(build.get("runs-on").is_none(), "{build}");
        assert!(build.get("steps").is_none(), "{build}");
    }

    /// The jobs wait on what they read: the draft and the publication on the
    /// builder, the publication on the draft and the guard, the record on
    /// the publication, and the withdrawal on everything the publication
    /// waits on and the publication itself, for its point of no return.
    #[test]
    fn the_jobs_wait_on_what_they_read() {
        let value = rendered();
        let jobs = value["jobs"].as_object().expect("jobs");
        let mut ids: Vec<&str> = jobs.keys().map(String::as_str).collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            [
                "build",
                "create-release",
                "guard",
                "publish",
                "record",
                "withdraw-release"
            ]
        );

        let mut create = needs(&jobs["create-release"]);
        create.sort_unstable();
        assert_eq!(create, ["build", "guard"]);
        let mut publish = needs(&jobs["publish"]);
        publish.sort_unstable();
        assert_eq!(publish, ["build", "create-release", "guard"]);
        let mut record = needs(&jobs["record"]);
        record.sort_unstable();
        assert_eq!(record, ["guard", "publish"]);
        let mut withdraw = needs(&jobs["withdraw-release"]);
        withdraw.sort_unstable();
        assert_eq!(withdraw, ["build", "create-release", "guard", "publish"]);
    }

    /// Scenario _The caller counts the builder's targets_: the publishing
    /// gate reads each target through the calling job's outputs, and the
    /// manifest is written in that job alone.
    #[test]
    fn the_publication_counts_the_builder_s_targets() {
        let value = rendered();
        let jobs = value["jobs"].as_object().expect("jobs");
        let cond = jobs["publish"]["if"].as_str().expect("if");
        for platform in Platform::iter() {
            let slug = platform.slug();
            assert!(
                cond.contains(&format!("needs.build.outputs.{slug} == '{slug}'")),
                "{slug}: {cond}"
            );
        }
        let writers: Vec<&String> = jobs
            .iter()
            .filter(|(_, job)| {
                job["steps"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|step| step["id"] == "write-manifest")
            })
            .map(|(name, _)| name)
            .collect();
        assert_eq!(writers, [jobs::PUBLISH_JOB]);
    }

    /// Scenario _A failed first preview undoes the cut_, as the workflow
    /// sees it: one `release undo` step with the tag and the commit the push
    /// was for, whose own logic deletes the series and reverts `main` for a
    /// `-pre.1`; the workflow wires nothing more for it.
    #[test]
    fn the_withdrawal_hands_the_cut_to_undo() {
        let value = rendered();
        let steps = value["jobs"]["withdraw-release"]["steps"]
            .as_array()
            .expect("steps");
        let undos: Vec<&Value> = steps
            .iter()
            .filter(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains("release undo"))
            })
            .collect();
        assert_eq!(undos.len(), 1);
        assert_eq!(undos[0]["env"]["COMMIT"], "${{ github.sha }}");
        assert!(
            steps.iter().all(|step| {
                let run = step["run"].as_str().unwrap_or("");
                !run.contains("release open-series") && !run.contains("release tag")
            }),
            "the withdrawal mints nothing"
        );
    }

    /// Scenario _A library tag starts nothing_: a library's tag matches
    /// neither the push trigger nor a channel's shape.
    #[test]
    fn a_group_tag_triggers_no_release() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = std::fs::read_to_string(root.join(crate::constants::GROUP_FILE)).unwrap();
        let file = crate::release_groups::GroupFile::parse(&text).unwrap();
        assert!(TAG_PATTERN.starts_with('v'));

        for (name, group) in &file.group {
            if group.tag == "v{version}" {
                continue;
            }
            let tag = group.tag.replace("{version}", "0.2.0");
            assert!(
                !tag.starts_with('v'),
                "{name}: {tag} would match the `{TAG_PATTERN}` trigger"
            );
        }
    }
}
