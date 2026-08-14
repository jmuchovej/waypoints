//! Builds a channel's release workflow.
//!
//! One shape for all three channels. The hand-written YAML had this shape three
//! times over — `stable.yaml`, `preview.yaml` and `nightly.yaml` differed only
//! in flavor strings, a tag pattern, and whether iOS was in scope — plus three
//! more reusable workflows underneath to hold the jobs.

use gh_workflow::{
    Event, Job, Level, Permissions, Push, Schedule, Workflow, WorkflowDispatch,
    WorkflowDispatchInput,
};
use indexmap::IndexMap;

use super::{
    WorkflowContext,
    bundle::{self, NamedJob},
    utils::{Channel, Platform, runners, steps, vars},
};

/// Nightly runs at 07:00 UTC (~03:00 US Eastern). `schedule` only fires on the
/// default branch, which is what a nightly should track.
const NIGHTLY_CRON: &str = "0 7 * * *";

use strum::IntoEnumIterator;

pub fn workflow(channel: Channel, context: &WorkflowContext) -> Workflow {
    let mut jobs: Vec<NamedJob> = Vec::new();

    if channel == Channel::Nightly {
        jobs.push(prepare());
    }

    jobs.push(bundle::desktop(channel, Platform::macOS));
    jobs.push(bundle::desktop(channel, Platform::Linux));
    jobs.push(bundle::desktop(channel, Platform::Windows));
    jobs.push(bundle::android(
        channel,
        &context.android_jdk,
        &context.android_ndk,
        &context.android_abis,
    ));
    if channel.ships_ios() {
        jobs.push(bundle::ios(channel));
    }

    // Every build waits on the nightly planner's verdict.
    if channel == Channel::Nightly {
        jobs = jobs
            .into_iter()
            .map(|named| match named.id.as_str() {
                "prepare" => named,
                _ => NamedJob {
                    job: named.job.add_needs("prepare"),
                    ..named
                },
            })
            .collect();
    }

    let workflow = Workflow::new(title(channel))
        .add_env(vars::FORCE_NODE24)
        .on(events(channel));

    let workflow = match channel {
        // A scheduled run and a manual or tag run must never bundle at once,
        // and cancelling one mid-publish would strand release artifacts.
        Channel::Nightly => workflow
            .permissions(Permissions::default().contents(Level::Write))
            .concurrency(vars::serialize("nightly")),
        _ => workflow,
    };

    jobs.into_iter().fold(workflow, |workflow, named| {
        workflow.add_job(named.id, named.job)
    })
}

fn title(channel: Channel) -> String {
    let mut name = channel.flavor().to_owned();
    name[..1].make_ascii_uppercase();
    name
}

fn events(channel: Channel) -> Event {
    let event = Event::default()
        .push(Push::default().add_tag(channel.tag_pattern()))
        .workflow_dispatch(dispatch(channel));

    match channel {
        Channel::Nightly => event.schedule(vec![Schedule::new(NIGHTLY_CRON)]),
        _ => event,
    }
}

fn dispatch(channel: Channel) -> WorkflowDispatch {
    WorkflowDispatch::default().inputs(IndexMap::from([
        (
            "tag".to_owned(),
            WorkflowDispatchInput::default()
                .description("Release tag (leave empty for build-only, no release)")
                .input_type("string")
                .set_default(""),
        ),
        (
            "platforms".to_owned(),
            WorkflowDispatchInput::default()
                .description("Platforms to build")
                .input_type("choice")
                .options(channel.dispatch_platforms())
                .set_default("All"),
        ),
    ]))
}

/// Resolves the nightly tag: derive it from the workspace version, skip the run
/// when nothing landed since the last nightly, and create the tag.
fn prepare() -> NamedJob {
    NamedJob {
        id: "prepare".to_owned(),
        job: Job::default()
            .name("Plan the nightly")
            .runs_on(runners::LINUX)
            .add_output("tag", "${{ steps.plan.outputs.tag }}")
            .add_output("should_build", "${{ steps.plan.outputs.should-build }}")
            // A shallow checkout is enough: the planner reads Cargo.toml from
            // the tree and reaches everything else over the API, so it needs no
            // local history and no committer identity to create the tag.
            .add_step(steps::checkout())
            .add_step(steps::setup_mise())
            .add_step(
                steps::cargo_xtask("nightly-plan", &[("--dispatch-tag", "${{ inputs.tag }}")])
                    .id("plan")
                    .add_env(("GH_TOKEN", vars::GITHUB_TOKEN)),
            ),
    }
}
