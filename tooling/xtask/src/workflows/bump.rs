// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Mints the refs a release cycle moves through, from one workflow: its
//! schedule bumps nightly, and its dispatch takes a target — a channel, or a
//! library group — the major flag, the floor, and a rehearsal flag that
//! defaults on. `release bump` infers the transition from the open series
//! read off the remote, so nothing here chooses one; the landing steps gate
//! on the transition it publishes.
//!
//! A dispatch rehearses unless it says otherwise, and a nightly dispatch
//! always does: the date is the store build number, and a day holds one
//! nightly, so only the schedule mints one. A dry run mints nothing, opens
//! nothing, and uploads what it would have written — the version writes as
//! patches, the refs, and the notes — as the run's artifact.
//!
//! A cut lands twice. Its main-side writes — each minted library's manifests
//! and `main`'s own bump — land first, as one pull request; the run waits for
//! that merge, tags each library at the merge commit, and opens the series
//! there, so a library tag always names a commit the planners' first-parent
//! walk contains. The series' version write then lands as the other
//! transitions' do, and the app's tag goes on its merge commit. A cut scoped
//! to one library stops after its tags. A nightly ends at its mint; every
//! tag's push starts the release.
//!
//! Refs are minted with the wp-botswain App token rather than `GITHUB_TOKEN`:
//! a tag pushed with the latter raises no `push` event, so the release
//! workflow would never run on it. `main` and the series branches are
//! protected, so every version write lands through a pull request gated like
//! any other. Each wait can outlast an App token, so a wait looks under
//! `GITHUB_TOKEN` and the steps after it mint a fresh token. One queued
//! concurrency group, shared with the release workflow: one release cycle
//! runs at a time, so a scheduled nightly waits out a release and cannot mint
//! from a `main` that run is about to revert.

use super::{MiseEnv, WorkflowInput, actions, actions::MiseSetup};
use super::{WorkflowContext, helpers, helpers::Environment, runners, steps, steps::Tools};
use gh_workflow::{
    Environment as GhEnvironment, Event, Expression, Job, Level, Permissions, Schedule, StepOutput,
    Workflow, WorkflowDispatch,
};

/// Nightly runs at 07:00 UTC (~03:00 US Eastern). `schedule` only fires on the
/// default branch, which is what a nightly should track.
const NIGHTLY_CRON: &str = "0 7 * * *";

/// The channels a bump takes as its target, offered before the library
/// groups; the planner takes one as `--channel`, and anything else as
/// `--group`.
pub(super) const CHANNELS: [&str; 3] = ["nightly", "preview", "stable"];

/// What a group bump may raise its version by at least.
const FLOORS: [&str; 4] = ["none", "patch", "minor", "major"];

/// Becomes the merge commit's body, so it says what the merge commit is.
pub(super) fn body() -> String {
    crate::planners::release::pull_request_body()
}

/// Whether the run rehearses: a dispatch unless it says otherwise, and a
/// nightly dispatch always; the schedule is the real thing.
const REHEARSING: &str =
    "github.event_name == 'workflow_dispatch' && (inputs.dry_run || inputs.target == 'nightly')";

/// The target the planner is asked for: nightly on the schedule, the
/// dispatch's otherwise. Through the environment — a group name is free text
/// as far as the shell is concerned.
const TARGET: &str = "${{ github.event_name == 'schedule' && 'nightly' || inputs.target }}";

/// The app's major is asked for, never computed. A boolean input cannot carry
/// text, so it is safe inline; on the schedule it is unset, so nothing.
const MAJOR_FLAG: &str = "${{ inputs.major && '--major' || '' }}";

/// The floor reaches the planner through the environment, empty where the
/// dispatch left it at `none` and on the schedule.
const FLOOR: &str = "${{ inputs.floor != 'none' && inputs.floor || '' }}";
const FLOOR_FLAG: &str = r#"${FLOOR:+--floor "$FLOOR"}"#;

/// Where a rehearsal renders, and what it uploads.
fn dry_run_flags() -> String {
    format!(r#"${{{{ {REHEARSING} && '--dry-run --into "${{RUNNER_TEMP}}/dry-run"' || '' }}}}"#)
}

/// `--channel` for a channel, `--group` for anything else; the schedule's
/// target is nightly.
fn kind_flag() -> String {
    format!(
        "${{{{ (github.event_name == 'schedule' || contains(fromJson('{channels}'), inputs.target)) && '--channel' || '--group' }}}}",
        channels = serde_json::to_string(&CHANNELS).expect("a list of strings")
    )
}

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let targets: Vec<&str> = CHANNELS
        .iter()
        .copied()
        .chain(context.groups.iter().map(String::as_str))
        .collect();
    let target = WorkflowInput::choices("target", targets, Some("nightly")).description(
        "What to bump: a channel, whose transition the planner infers from the open series, or one library group",
    );
    let major = WorkflowInput::bool("major", Some(false))
        .description("Cut toward the app's next major; refused where the bump would not cut");
    let floor = WorkflowInput::choices("floor", FLOORS.to_vec(), Some("none"))
        .description("The least a group bump moves its version by; never lowers it");
    let dry_run = WorkflowInput::bool("dry_run", Some(true)).description(
        "Rehearse: plan, render the writes and refs as an artifact, and mint nothing; a nightly dispatch always does",
    );

    let real = || Expression::new(format!("!({REHEARSING})"));
    let rehearsing = || Expression::new(REHEARSING);

    let app_token =
        steps::app_token(&[("contents", "write"), ("pull-requests", "write")]).if_condition(real());
    let token = StepOutput::new_trusted(&app_token, "token");
    let registry = steps::cargo_registry(steps::CARGO_HOME);

    let planner = steps::cargo_xtask(
        "release bump",
        &[
            (&kind_flag(), Some("$TARGET")),
            (MAJOR_FLAG, None),
            (FLOOR_FLAG, None),
            (&dry_run_flags(), None),
        ],
    )
    .id("bump")
    .add_env(("GH_TOKEN", token.to_string()))
    // git-cliff renders a rehearsal's notes and reads `GITHUB_TOKEN` for the
    // pull-request metadata the template names; anonymous, it meets the
    // API's rate limit. The planner's own calls prefer `GH_TOKEN`.
    .add_env(("GITHUB_TOKEN", helpers::GITHUB_TOKEN))
    .add_env(("TARGET", TARGET))
    .add_env(("FLOOR", FLOOR));
    let output = |name| StepOutput::new_trusted(&planner, name);
    let transition = output("transition");
    let tag = output("tag");
    let group_tags = output("group_tags");
    let version = output("version");
    let cutting = || Expression::new(format!("!({REHEARSING}) && {} == 'cut'", transition.expr()));
    // A nightly publishes the tag it already minted, so everything past the
    // planner stands down on it.
    let past_a_mint = || {
        Expression::new(format!(
            "!({REHEARSING}) && {} != 'nightly'",
            transition.expr()
        ))
    };
    let landing = |number: &StepOutput| {
        Expression::new(format!("!({REHEARSING}) && {} != ''", number.expr()))
    };

    // A cut lands on `main` first: the planner left the main-side manifests
    // in the tree and published that landing, with the summary's headers in a
    // body file.
    let main =
        steps::land_write_from_file(&planner, &token, "main-pull-request").if_condition(cutting());
    let main_number = StepOutput::new_trusted(&main, "pull-request-number");
    let merge_main = steps::merge(&main_number, &token, Tools::Mise)
        .id("merge-main")
        .if_condition(Expression::new(format!(
            "!({REHEARSING}) && {} == 'cut' && {} != ''",
            transition.expr(),
            main_number.expr()
        )));
    let main_commit = StepOutput::new_trusted(&merge_main, "merge_commit");

    // The series phase runs under a token minted after the main merge, which
    // can outlast the first; the other transitions have no wait before it.
    let series_token = steps::app_token(&[("contents", "write"), ("pull-requests", "write")])
        .name("Mint a wp-botswain token for the series")
        .id("series-token")
        .if_condition(past_a_mint());
    let series_token_output = StepOutput::new_trusted(&series_token, "token");

    let minting = || {
        Expression::new(format!(
            "!({REHEARSING}) && {} == 'cut' && {} != ''",
            transition.expr(),
            group_tags.expr()
        ))
    };
    let tag_groups = steps::tag(&group_tags, &main_commit, &series_token_output)
        .id("tag-groups")
        .if_condition(minting());

    // Every transition but a cut wrote on its series in the planner step; a
    // cut opens its series here, from the merge commit, and a cut scoped to
    // one library opens none. A nightly's tag is already minted, so it ends
    // there.
    let on_a_series = || {
        Expression::new(format!(
            "!({REHEARSING}) && {} != 'nightly' && {} != ''",
            transition.expr(),
            tag.expr()
        ))
    };
    let opening = || {
        Expression::new(format!(
            "!({REHEARSING}) && {} == 'cut' && {} != ''",
            transition.expr(),
            tag.expr()
        ))
    };
    let open_series =
        steps::open_series(&version, &main_commit, &series_token_output).if_condition(opening());
    let from_series_or_planner = |name: &str| {
        format!("${{{{ steps.open-series.outputs.{name} || steps.bump.outputs.{name} }}}}")
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
    let merge_series = steps::merge(&series_number, &series_token_output, Tools::Mise)
        .id("merge-series")
        .if_condition(landing(&series_number));
    let merge_commit = StepOutput::new_trusted(&merge_series, "merge_commit");
    let tag_token = steps::app_token(&[("contents", "write")])
        .name("Mint a wp-botswain token for the tag")
        .id("tag-token")
        .if_condition(on_a_series());
    let tag_step = steps::tag(
        &tag,
        &merge_commit,
        &StepOutput::new_trusted(&tag_token, "token"),
    )
    .id("tag-release")
    .if_condition(on_a_series());

    let upload = steps::upload_rendering("dry-run-bump".to_owned(), rehearsing());

    let bump = Job::new("bump the release cycle")
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
        // release to HEAD, and a nightly from the last nightly, and in a
        // shallow clone the tip is a root commit that appears to touch every
        // path.
        .add_step(
            steps::checkout()
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true)),
        )
        .add_step(actions::setup_mise(
            MiseSetup::default(),
            &context.mise_version,
        ))
        // The cost here is compiling xtask, and the build jobs' registry
        // snapshot and compiler cache answer most of it.
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
        .add_step(merge_main)
        .add_step(series_token)
        .add_step(tag_groups)
        .add_step(open_series)
        .add_step(series)
        .add_step(merge_series)
        .add_step(tag_token)
        .add_step(tag_step);

    helpers::named_workflow("bump the release cycle")
        .on(Event::default()
            .schedule(vec![Schedule::new(NIGHTLY_CRON)])
            .workflow_dispatch(WorkflowDispatch::default().inputs([
                (target.name.into(), target.input()),
                (major.name.into(), major.input()),
                (floor.name.into(), floor.input()),
                (dry_run.name.into(), dry_run.input()),
            ])))
        .concurrency(helpers::serialize(helpers::RELEASE_CYCLE))
        .add_job("bump", bump)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn rendered() -> Value {
        serde_json::to_value(workflow(&WorkflowContext::fixture())).unwrap()
    }

    fn steps() -> Vec<Value> {
        rendered()["jobs"]["bump"]["steps"]
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

    /// Scenario _One workflow bumps every channel and every group_: the
    /// schedule and a dispatch whose target is the three channels and then
    /// every library group the group file names, the major flag, the floor,
    /// and a rehearsal flag that defaults on; one queued concurrency group,
    /// the release workflow's.
    #[test]
    fn each_workflow_offers_its_dispatch_inputs() {
        let value = rendered();
        assert_eq!(
            value["on"]["schedule"],
            serde_json::json!([{ "cron": NIGHTLY_CRON }])
        );
        let inputs = value["on"]["workflow_dispatch"]["inputs"]
            .as_object()
            .expect("inputs");
        let mut names: Vec<&str> = inputs.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, ["dry_run", "floor", "major", "target"]);

        let default = |name: &str| inputs[name]["default"].to_string().replace('"', "");
        assert_eq!(default("target"), "nightly");
        assert_eq!(default("major"), "false");
        assert_eq!(default("floor"), "none");
        assert_eq!(default("dry_run"), "true");
        assert_eq!(inputs["target"]["type"], "choice");
        let context = WorkflowContext::fixture();
        let expected: Vec<&str> = CHANNELS
            .iter()
            .copied()
            .chain(context.groups.iter().map(String::as_str))
            .collect();
        let offered: Vec<&str> = inputs["target"]["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(|option| option.as_str().unwrap())
            .collect();
        assert_eq!(offered, expected);
        assert_eq!(inputs["floor"]["type"], "choice");

        assert_eq!(value["concurrency"]["group"], helpers::RELEASE_CYCLE);
        assert_eq!(value["concurrency"]["cancel-in-progress"], false);
    }

    /// Scenarios _The schedule mints a nightly_ and _A nightly dispatch is
    /// always dry_: the schedule's planner step is `--channel nightly` with
    /// no `--dry-run`, and a dispatch's rehearses when asked or when its
    /// target is nightly — one expression, read once.
    #[test]
    fn the_schedule_mints_and_a_dispatched_nightly_rehearses() {
        let steps = steps();
        let planner = &steps[position(&steps, "bump")];
        let run = planner["run"].as_str().unwrap();
        assert!(run.starts_with("cargo ci-task release bump "), "{run}");
        assert!(
            run.contains("github.event_name == 'schedule' || contains(fromJson("),
            "{run}"
        );
        assert!(
            run.contains("&& '--channel' || '--group' }} \"$TARGET\""),
            "{run}"
        );
        assert_eq!(planner["env"]["TARGET"], TARGET);
        assert!(
            TARGET.contains("github.event_name == 'schedule' && 'nightly'"),
            "{TARGET}"
        );
        assert!(run.contains(MAJOR_FLAG), "{run}");
        assert!(run.contains(FLOOR_FLAG), "{run}");
        assert_eq!(planner["env"]["FLOOR"], FLOOR);
        assert!(!run.contains("inputs.floor"), "{run}");
        assert!(run.contains(&dry_run_flags()), "{run}");
        assert!(
            dry_run_flags().starts_with(&format!("${{{{ {REHEARSING} && '--dry-run")),
            "{}",
            dry_run_flags()
        );
        assert!(REHEARSING.contains("inputs.target == 'nightly'"));
        assert!(REHEARSING.starts_with("github.event_name == 'workflow_dispatch'"));
    }

    /// A tag made before the merge names a commit the planners' first-parent
    /// walk never reaches.
    #[test]
    fn the_tag_waits_for_the_merge_commit() {
        let steps = steps();
        let tag = position(&steps, "tag-release");

        assert!(position(&steps, "merge-series") < tag);
        assert_eq!(
            steps[tag]["env"]["COMMIT"],
            "${{ steps.merge-series.outputs.merge_commit }}"
        );
        assert_eq!(steps[tag]["env"]["TAGS"], "${{ steps.bump.outputs.tag }}");
    }

    /// An App token lasts an hour, and the required checks can take most of it.
    /// The merge enables under the App token and waits under the runner's own,
    /// and the tag is minted under a token minted after the wait: one that
    /// expired mid-wait would land the write and lose the tag.
    #[test]
    fn the_tag_gets_a_token_minted_after_the_wait() {
        let steps = steps();
        let merge = position(&steps, "merge-series");
        assert_eq!(steps[merge]["env"]["GITHUB_TOKEN"], helpers::GITHUB_TOKEN);
        assert_eq!(
            steps[merge]["env"]["GH_TOKEN"],
            "${{ steps.series-token.outputs.token }}"
        );

        assert!(merge < position(&steps, "tag-token"));
        assert_eq!(
            steps[position(&steps, "tag-release")]["env"]["GH_TOKEN"],
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
    /// starts from the tree the cut tagged.
    #[test]
    fn a_cut_lands_main_then_tags_then_opens_the_series() {
        let steps = steps();
        let order = [
            position(&steps, "bump"),
            position(&steps, "main-pull-request"),
            position(&steps, "merge-main"),
            position(&steps, "series-token"),
            position(&steps, "tag-groups"),
            position(&steps, "open-series"),
            position(&steps, "series-pull-request"),
            position(&steps, "merge-series"),
            position(&steps, "tag-release"),
        ];
        assert!(order.is_sorted(), "{order:?}");

        let tag_groups = &steps[position(&steps, "tag-groups")];
        assert_eq!(
            tag_groups["env"]["COMMIT"],
            "${{ steps.merge-main.outputs.merge_commit }}"
        );
        assert_eq!(
            tag_groups["env"]["TAGS"],
            "${{ steps.bump.outputs.group_tags }}"
        );
        assert_eq!(
            tag_groups["env"]["GH_TOKEN"],
            "${{ steps.series-token.outputs.token }}"
        );
        let open = &steps[position(&steps, "open-series")];
        assert_eq!(
            open["env"]["COMMIT"],
            "${{ steps.merge-main.outputs.merge_commit }}"
        );
        assert_eq!(open["env"]["VERSION"], "${{ steps.bump.outputs.version }}");
    }

    /// The main-side landing and the series opening are a cut's alone, read
    /// off the transition the planner inferred; the series landing serves
    /// every transition that wrote on a series, reading the series opener's
    /// outputs where a cut produced them and the planner's otherwise, and a
    /// nightly — which publishes no tag to land — ends at the mint.
    #[test]
    fn only_a_cut_lands_on_main_and_opens_a_series() {
        let steps = steps();
        for id in [
            "main-pull-request",
            "merge-main",
            "open-series",
            "tag-groups",
        ] {
            let cond = steps[position(&steps, id)]["if"].as_str().unwrap_or("");
            assert!(
                cond.contains("steps.bump.outputs.transition == 'cut'"),
                "{id}: {cond}"
            );
        }
        assert!(
            steps[position(&steps, "tag-groups")]["if"]
                .as_str()
                .unwrap()
                .contains("steps.bump.outputs.group_tags != ''")
        );

        let series = &steps[position(&steps, "series-pull-request")];
        for key in ["branch", "title", "add-paths"] {
            let value = series["with"][key].as_str().unwrap();
            assert!(
                value.contains("steps.open-series.outputs."),
                "{key}: {value}"
            );
            assert!(value.contains("|| steps.bump.outputs."), "{key}: {value}");
        }
        assert_eq!(
            series["if"],
            format!(
                "!({REHEARSING}) && steps.bump.outputs.transition != 'nightly' && steps.bump.outputs.tag != ''"
            )
        );
    }

    /// A scheduled nightly is minted by the planner step itself and publishes
    /// that tag, so every step past the planner — the series token, the
    /// landing, and the tag — stands down on `transition=nightly` rather than
    /// landing an empty head.
    #[test]
    fn a_nightly_ends_at_its_mint() {
        let steps = steps();
        for id in [
            "series-token",
            "series-pull-request",
            "tag-token",
            "tag-release",
        ] {
            let cond = steps[position(&steps, id)]["if"].as_str().unwrap_or("");
            assert!(
                cond.contains("steps.bump.outputs.transition != 'nightly'"),
                "{id} would run on a nightly: {cond}"
            );
        }
    }

    /// git-cliff renders a rehearsal's notes under the runner's token; the
    /// planner's own calls keep preferring the App token.
    #[test]
    fn the_planner_hands_git_cliff_the_runner_s_token() {
        let steps = steps();
        let planner = &steps[position(&steps, "bump")];
        assert_eq!(planner["env"]["GITHUB_TOKEN"], helpers::GITHUB_TOKEN);
        assert_eq!(
            planner["env"]["GH_TOKEN"],
            "${{ steps.app-token.outputs.token }}"
        );
    }

    /// A resume (design D9) publishes `transition=resume` with a tag and a
    /// series landing, so the series steps run on it as on an advance, and
    /// every cut-only step — which reads `== 'cut'` — stands down.
    #[test]
    fn a_resume_lands_on_its_series_and_cuts_nothing() {
        let steps = steps();
        let series_steps = [
            "series-pull-request",
            "merge-series",
            "tag-token",
            "tag-release",
        ];
        for id in series_steps {
            let cond = steps[position(&steps, id)]["if"].as_str().unwrap_or("");
            assert!(
                !cond.contains("transition =="),
                "{id} names a transition, so a resume would skip it: {cond}"
            );
        }
        for id in [
            "main-pull-request",
            "merge-main",
            "open-series",
            "tag-groups",
        ] {
            let cond = steps[position(&steps, id)]["if"].as_str().unwrap_or("");
            assert!(
                cond.contains("== 'cut'"),
                "{id} would run on a resume: {cond}"
            );
        }
    }

    /// The summary's per-group headers span lines, so the main pull request
    /// reads its body from the file the planner wrote.
    #[test]
    fn the_main_pull_request_reads_its_body_from_the_planner_s_file() {
        let steps = steps();
        let main = &steps[position(&steps, "main-pull-request")];
        assert_eq!(
            main["with"]["body-path"],
            "${{ steps.bump.outputs.body_path }}"
        );
        assert!(main["with"].get("body").is_none());
        assert_eq!(main["with"]["branch"], "${{ steps.bump.outputs.head }}");
    }

    /// Each pull request is landed by a merge step reading its own number.
    #[test]
    fn each_pull_request_merges_itself() {
        let steps = steps();
        let numbers: Vec<&str> = steps
            .iter()
            .filter(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains("release merge"))
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

    /// Every step that mints, lands, or authenticates stands down on a
    /// rehearsal; the rendering is uploaded only then.
    #[test]
    fn every_landing_step_stands_down_on_a_dry_run() {
        let steps = steps();
        let real = format!("!({REHEARSING})");
        for id in [
            "main-pull-request",
            "merge-main",
            "series-token",
            "tag-groups",
            "open-series",
            "series-pull-request",
            "merge-series",
            "tag-token",
            "tag-release",
            "app-token",
        ] {
            let cond = steps[position(&steps, id)]["if"].as_str().unwrap_or("");
            assert!(cond.starts_with(&real), "{id}: {cond}");
        }
        for command in ["release tag --tags", "release open-series", "release merge"] {
            let cond = steps[running(&steps, command)]["if"].as_str().unwrap_or("");
            assert!(cond.starts_with(&real), "{command}: {cond}");
        }
        let credentials = steps
            .iter()
            .find(|step| step["name"] == "Resolve the botswain secrets")
            .expect("the credentials step");
        assert_eq!(credentials["if"], real);

        let upload = &steps[position(&steps, "upload-dry-run")];
        assert_eq!(upload["if"], REHEARSING);
        assert_eq!(upload["with"]["name"], "dry-run-bump");
        assert_eq!(upload["with"]["path"], steps::DRY_RUN_ARTIFACT_PATH);
        assert!(!steps::DRY_RUN_ARTIFACT_PATH.contains("${RUNNER_TEMP}"));
    }
}
