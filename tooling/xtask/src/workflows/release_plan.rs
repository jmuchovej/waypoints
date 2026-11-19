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
//! A dispatch rehearses unless it says otherwise: `dry_run` defaults to true,
//! and a dry run mints nothing, opens nothing, and uploads what it would have
//! written — the version writes as patches, the refs, and the notes — as the
//! run's artifact.
//!
//! A cut lands twice. Its main-side writes — each minted library's manifests
//! and `main`'s own bump — land first, as one pull request; the run waits for
//! that merge, tags each library at the merge commit, and opens the series
//! there, so a library tag always names a commit the planners' first-parent
//! walk contains. The series' version write then lands as the other
//! transitions' do, and the app's tag goes on its merge commit. A cut scoped
//! to one library stops after its tags.
//!
//! Nightlies are minted by [`super::release`]'s `prepare` job instead — their
//! version is a timestamp, not a claim about content.
//!
//! Refs are minted with the wp-botswain App token rather than `GITHUB_TOKEN`:
//! a tag pushed with the latter raises no `push` event, so neither the
//! channel's release workflow nor the release index would run on it.
//!
//! `main` and the series branches are protected, so every version write lands
//! through a pull request gated like any other. Each wait can outlast an App
//! token, so a wait looks under `GITHUB_TOKEN` and the steps after it mint a
//! fresh token.

use super::{MiseEnv, WorkflowInput};
use super::{WorkflowContext, helpers, helpers::Environment, runners, steps, steps::Tools};
use gh_workflow::{
    Environment as GhEnvironment, Event, Expression, Job, Level, Permissions, StepOutput, Workflow,
    WorkflowDispatch,
};

/// Becomes the merge commit's body, so it says what the merge commit is.
pub(super) fn body() -> String {
    crate::planners::release::pull_request_body()
}

/// Where a dry run renders, and what it uploads.
const DRY_RUN_FLAGS: &str =
    r#"${{ inputs.dry_run && '--dry-run --into "${RUNNER_TEMP}/dry-run"' || '' }}"#;

/// The app's major is asked for, never computed. A boolean input cannot carry
/// text, so it is safe inline.
const MAJOR_FLAG: &str = "${{ inputs.major && '--major' || '' }}";

/// A group and a floor reach the planner through the environment: the group
/// is free text, which never belongs on a command line as written.
const GROUP_FLAGS: &str = r#"${GROUP:+--group "$GROUP"} ${FLOOR:+--floor "$FLOOR"}"#;

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let transition = WorkflowInput::choices(
        "transition",
        crate::releases::version::transitions(),
        None::<String>,
    )
    .description("Which release-cycle transition to perform");
    let dry_run = WorkflowInput::bool("dry_run", Some(true))
        .description("Rehearse: plan, render the writes and refs as an artifact, and mint nothing");
    let major = WorkflowInput::bool("major", Some(false)).description(
        "Cut toward the app's next major; without it, a cut after a `Covenant: break` is refused",
    );
    let group = WorkflowInput::string("group", None)
        .description("Cut one library group alone: its tag on main, no series, nothing else moved");
    let floor = WorkflowInput::string("floor", None)
        .description("The least a group-scoped cut bumps: major, minor, or patch");

    let real = || Expression::new(format!("!{}", dry_run.expr()));
    let rehearsing = || Expression::new(dry_run.expr());
    let cutting = || {
        Expression::new(format!(
            "!{} && {} == 'cut'",
            dry_run.expr(),
            transition.expr()
        ))
    };

    let app_token =
        steps::app_token(&[("contents", "write"), ("pull-requests", "write")]).if_condition(real());
    let token = StepOutput::new_trusted(&app_token, "token");
    let registry = steps::cargo_registry(steps::CARGO_HOME);

    let planner = steps::cargo_xtask(
        "release-plan",
        &[
            ("${{ inputs.transition }}", None),
            (MAJOR_FLAG, None),
            (GROUP_FLAGS, None),
            (DRY_RUN_FLAGS, None),
        ],
    )
    .id("release-plan")
    .add_env(("GH_TOKEN", token.to_string()))
    .add_env(("GROUP", "${{ inputs.group }}"))
    .add_env(("FLOOR", "${{ inputs.floor }}"));
    let output = |name| StepOutput::new_trusted(&planner, name);
    let tag = output("tag");
    let group_tags = output("group_tags");
    let version = output("version");
    let landing = |number: &StepOutput| {
        Expression::new(format!("!{} && {} != ''", dry_run.expr(), number.expr()))
    };

    // A cut lands on `main` first: the planner left the main-side manifests
    // in the tree and published that landing, with the summary's headers in a
    // body file.
    let main =
        steps::land_write_from_file(&planner, &token, "main-pull-request").if_condition(cutting());
    let main_number = StepOutput::new_trusted(&main, "pull-request-number");
    let auto_merge_main =
        steps::auto_merge(&main_number, &token, Tools::Mise).if_condition(landing(&main_number));
    let wait_main = steps::wait_for_merge(&main_number)
        .id("wait-for-main")
        .if_condition(cutting());
    let main_commit = StepOutput::new_trusted(&wait_main, "merge_commit");

    // The series phase runs under a token minted after the main wait, which
    // can outlast the first; the other transitions have no wait before it.
    let series_token = steps::app_token(&[("contents", "write"), ("pull-requests", "write")])
        .name("Mint a wp-botswain token for the series")
        .id("series-token")
        .if_condition(real());
    let series_token_output = StepOutput::new_trusted(&series_token, "token");

    let minting = || {
        Expression::new(format!(
            "!{} && {} == 'cut' && {} != ''",
            dry_run.expr(),
            transition.expr(),
            group_tags.expr()
        ))
    };
    let tag_groups =
        steps::tag_groups(&group_tags, &main_commit, &series_token_output).if_condition(minting());

    // Every transition but a cut wrote on its series in the planner step; a
    // cut opens its series here, from the merge commit, and a cut scoped to
    // one library opens none.
    let on_a_series = || Expression::new(format!("!{} && {} != ''", dry_run.expr(), tag.expr()));
    let opening = || {
        Expression::new(format!(
            "!{} && {} == 'cut' && {} != ''",
            dry_run.expr(),
            transition.expr(),
            tag.expr()
        ))
    };
    let open_series =
        steps::open_series(&version, &main_commit, &series_token_output).if_condition(opening());
    let from_series_or_planner = |name: &str| {
        format!("${{{{ steps.open-series.outputs.{name} || steps.release-plan.outputs.{name} }}}}")
    };
    let series = steps::create_pull_request(
        &series_token_output,
        &from_series_or_planner("head"),
        &from_series_or_planner("title"),
        &body(),
        &[from_series_or_planner("paths")],
    )
    .id("series-pull-request")
    .if_condition(on_a_series());
    let series_number = StepOutput::new_trusted(&series, "pull-request-number");
    let auto_merge_series = steps::auto_merge(&series_number, &series_token_output, Tools::Mise)
        .if_condition(landing(&series_number));

    let wait = steps::wait_for_merge(&series_number).if_condition(on_a_series());
    let merge_commit = StepOutput::new_trusted(&wait, "merge_commit");
    let tag_token = steps::app_token(&[("contents", "write")])
        .name("Mint a wp-botswain token for the tag")
        .id("tag-token")
        .if_condition(on_a_series());
    let tag_step = steps::tag_merge_commit(
        &tag,
        &merge_commit,
        &StepOutput::new_trusted(&tag_token, "token"),
    )
    .if_condition(on_a_series());

    let upload = steps::upload_rendering("dry-run-release-plan".to_owned(), rehearsing());

    let plan = Job::new("plan the transition")
        // A `cargo xtask` run and nothing more.
        .add_env(MiseEnv::select(&[MiseEnv::Xtask]))
        // Only what `checkout` and the waits need: every write goes through an
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
        // release to HEAD, and in a shallow clone the tip is a root commit that
        // appears to touch every path — see the nightly planner.
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
        // A dry run mints nothing, so it authenticates as nobody.
        .add_step(steps::botswain_credentials(Tools::Mise).if_condition(real()))
        .add_step(app_token)
        .add_step(steps::with_sccache(planner))
        // Before the pull requests: the waits below are long, and this stops
        // the server holding the cache credential.
        .add_step(steps::sccache_stats(Tools::Mise))
        .add_step(steps::cache_save(&registry, Tools::Mise))
        .add_step(upload)
        .add_step(main)
        .add_step(auto_merge_main)
        .add_step(wait_main)
        .add_step(series_token)
        .add_step(tag_groups)
        .add_step(open_series)
        .add_step(series)
        .add_step(auto_merge_series)
        .add_step(wait)
        .add_step(tag_token)
        .add_step(tag_step);

    helpers::named_workflow("mint the refs for a release-cycle transition")
        .on(
            Event::default().workflow_dispatch(WorkflowDispatch::default().inputs([
                (transition.name.into(), transition.input()),
                (dry_run.name.into(), dry_run.input()),
                (major.name.into(), major.input()),
                (group.name.into(), group.input()),
                (floor.name.into(), floor.input()),
            ])),
        )
        // A half-applied transition leaves the branch and its tag disagreeing.
        .concurrency(helpers::serialize("release-plan"))
        .add_job("release-plan", plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn rendered() -> Value {
        serde_json::to_value(workflow(&WorkflowContext::fixture())).unwrap()
    }

    fn steps() -> Vec<Value> {
        rendered()["jobs"]["release-plan"]["steps"]
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
        let tag = running(&steps, "release-land tag --tags \"${TAG}\"");

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
            steps[running(&steps, "release-land tag --tags \"${TAG}\"")]["env"]["GH_TOKEN"],
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

    /// A cut lands on `main` first and waits for it, mints the library tags
    /// at that merge commit, opens the series there, and only then lands the
    /// series: a library tag names a commit the walk contains, and a series
    /// starts from the tree its pins describe.
    #[test]
    fn a_cut_lands_main_then_tags_then_opens_the_series() {
        let steps = steps();
        let order = [
            position(&steps, "release-plan"),
            position(&steps, "main-pull-request"),
            position(&steps, "wait-for-main"),
            position(&steps, "series-token"),
            position(&steps, "tag-groups"),
            position(&steps, "open-series"),
            position(&steps, "series-pull-request"),
            position(&steps, "wait-for-merge"),
            running(&steps, "release-land tag --tags \"${TAG}\""),
        ];
        assert!(order.is_sorted(), "{order:?}");

        let tag_groups = &steps[position(&steps, "tag-groups")];
        assert_eq!(
            tag_groups["env"]["COMMIT"],
            "${{ steps.wait-for-main.outputs.merge_commit }}"
        );
        assert_eq!(
            tag_groups["env"]["TAGS"],
            "${{ steps.release-plan.outputs.group_tags }}"
        );
        assert_eq!(
            tag_groups["env"]["GH_TOKEN"],
            "${{ steps.series-token.outputs.token }}"
        );
        let open = &steps[position(&steps, "open-series")];
        assert_eq!(
            open["env"]["COMMIT"],
            "${{ steps.wait-for-main.outputs.merge_commit }}"
        );
        assert_eq!(
            open["env"]["VERSION"],
            "${{ steps.release-plan.outputs.version }}"
        );
    }

    /// The main-side landing and the series opening are a cut's alone; the
    /// series landing serves every transition, reading the series opener's
    /// outputs where a cut produced them and the planner's otherwise.
    #[test]
    fn only_a_cut_lands_on_main_and_opens_a_series() {
        let steps = steps();
        for id in [
            "main-pull-request",
            "wait-for-main",
            "open-series",
            "tag-groups",
        ] {
            let cond = steps[position(&steps, id)]["if"].as_str().unwrap_or("");
            assert!(cond.contains("inputs.transition == 'cut'"), "{id}: {cond}");
        }
        assert!(
            steps[position(&steps, "tag-groups")]["if"]
                .as_str()
                .unwrap()
                .contains("steps.release-plan.outputs.group_tags != ''")
        );

        let series = &steps[position(&steps, "series-pull-request")];
        for key in ["branch", "title", "add-paths"] {
            let value = series["with"][key].as_str().unwrap();
            assert!(
                value.contains("steps.open-series.outputs."),
                "{key}: {value}"
            );
            assert!(
                value.contains("|| steps.release-plan.outputs."),
                "{key}: {value}"
            );
        }
        assert_eq!(
            series["if"],
            "!inputs.dry_run && steps.release-plan.outputs.tag != ''"
        );
    }

    /// The summary's per-group headers span lines, so the main pull request
    /// reads its body from the file the planner wrote.
    #[test]
    fn the_main_pull_request_reads_its_body_from_the_planner_s_file() {
        let steps = steps();
        let main = &steps[position(&steps, "main-pull-request")];
        assert_eq!(
            main["with"]["body-path"],
            "${{ steps.release-plan.outputs.body_path }}"
        );
        assert!(main["with"].get("body").is_none());
        assert_eq!(
            main["with"]["branch"],
            "${{ steps.release-plan.outputs.head }}"
        );
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
                "${{ steps.main-pull-request.outputs.pull-request-number }}",
                "${{ steps.series-pull-request.outputs.pull-request-number }}",
            ]
        );
    }

    /// The compiler cache's credential should not sit in a server for the length
    /// of the waits.
    #[test]
    fn the_cache_server_stops_before_the_first_landing() {
        let steps = steps();
        assert!(position(&steps, "sccache-stats") < position(&steps, "main-pull-request"));
    }

    /// The dispatch rehearses unless told otherwise, and the planner is handed
    /// the flags and the directory only then. The free-text inputs reach the
    /// planner through its environment, never the command line as written.
    #[test]
    fn a_dispatch_rehearses_by_default() {
        let value = rendered();
        let inputs = &value["on"]["workflow_dispatch"]["inputs"];
        assert_eq!(
            inputs["dry_run"]["default"].to_string().replace('"', ""),
            "true"
        );
        assert!(inputs["transition"].is_object());
        assert_eq!(inputs["major"]["type"], "boolean");
        assert_eq!(inputs["group"]["type"], "string");
        assert_eq!(inputs["floor"]["type"], "string");

        let steps = steps();
        let planner = &steps[position(&steps, "release-plan")];
        let run = planner["run"].as_str().unwrap();
        assert!(run.contains(DRY_RUN_FLAGS), "{run}");
        assert!(run.contains(r#"--into "${RUNNER_TEMP}/dry-run""#), "{run}");
        assert!(run.contains(MAJOR_FLAG), "{run}");
        assert!(run.contains(GROUP_FLAGS), "{run}");
        assert!(!run.contains("inputs.group"), "{run}");
        assert_eq!(planner["env"]["GROUP"], "${{ inputs.group }}");
        assert_eq!(planner["env"]["FLOOR"], "${{ inputs.floor }}");
    }

    /// Every step that mints, lands, or authenticates stands down on a dry run;
    /// the rendering is uploaded only then.
    #[test]
    fn every_landing_step_stands_down_on_a_dry_run() {
        let steps = steps();
        for id in [
            "main-pull-request",
            "wait-for-main",
            "series-token",
            "tag-groups",
            "open-series",
            "series-pull-request",
            "wait-for-merge",
            "tag-token",
            "app-token",
        ] {
            let cond = steps[position(&steps, id)]["if"].as_str().unwrap_or("");
            assert!(cond.starts_with("!inputs.dry_run"), "{id}: {cond}");
        }
        for command in [
            "release-land tag --tags \"${TAG}\"",
            "release-land tag --tags \"${TAGS}\"",
            "release-land open-series",
            "release-land auto-merge",
        ] {
            let cond = steps[running(&steps, command)]["if"].as_str().unwrap_or("");
            assert!(cond.starts_with("!inputs.dry_run"), "{command}: {cond}");
        }
        let credentials = steps
            .iter()
            .find(|step| step["name"] == "Resolve the botswain secrets")
            .expect("the credentials step");
        assert_eq!(credentials["if"], "!inputs.dry_run");

        let upload = &steps[position(&steps, "upload-dry-run")];
        assert_eq!(upload["if"], "inputs.dry_run");
        assert_eq!(upload["with"]["name"], "dry-run-release-plan");
        assert_eq!(upload["with"]["path"], steps::DRY_RUN_ARTIFACT_PATH);
        assert!(!steps::DRY_RUN_ARTIFACT_PATH.contains("${RUNNER_TEMP}"));
    }

    /// A library tag is a source ref, not a release: it matches none of the
    /// `v…` patterns the release workflows and the index trigger on, so a
    /// group-scoped cut starts no run and no run can withdraw its tag.
    #[test]
    fn a_group_tag_triggers_no_release_or_index_run() {
        use crate::Channel;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = std::fs::read_to_string(root.join(crate::release_groups::GROUP_FILE)).unwrap();
        let file = crate::release_groups::GroupFile::parse(&text).unwrap();

        let patterns: Vec<&str> = [Channel::Nightly, Channel::Preview, Channel::Stable]
            .into_iter()
            .filter_map(|channel| channel.tag_pattern())
            .chain([super::super::release_index::TAG_PATTERN])
            .collect();
        assert!(!patterns.is_empty());
        for pattern in &patterns {
            assert!(pattern.starts_with('v'), "{pattern}");
        }

        for (name, group) in &file.group {
            if group.tag == "v{version}" {
                continue;
            }
            let tag = group.tag.replace("{version}", "0.2.0");
            assert!(
                !tag.starts_with('v'),
                "{name}: {tag} would match a `v…` trigger"
            );
        }
    }
}
