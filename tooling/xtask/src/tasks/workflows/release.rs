//! Builds a channel's release workflow.
//!
//! One shape for all three channels. The hand-written YAML had this shape three
//! times over — `stable.yaml`, `preview.yaml` and `nightly.yaml` differed only
//! in flavor strings, a tag pattern, and whether iOS was in scope — plus three
//! more reusable workflows underneath to hold the jobs.

use gh_workflow::{
    Event, Expression, Job, Level, Permissions, Push, Schedule, Workflow, WorkflowDispatch,
    WorkflowDispatchInput,
};
use indexmap::IndexMap;

use super::{
    WorkflowContext,
    bundle::{self, NamedJob},
    tests,
    utils::{Channel, Platform, runners, steps, vars},
};

/// Nightly runs at 07:00 UTC (~03:00 US Eastern). `schedule` only fires on the
/// default branch, which is what a nightly should track.
const NIGHTLY_CRON: &str = "0 7 * * *";

/// The nightly planner's verdict on whether anything changed since last night.
const SHOULD_BUILD: &str = "needs.prepare.outputs.should_build == 'true'";

use strum::IntoEnumIterator;

pub fn workflow(channel: Channel, context: &WorkflowContext) -> Workflow {
    let mut jobs: Vec<NamedJob> = Vec::new();

    if channel == Channel::Nightly {
        jobs.push(prepare());
    }

    // Nothing ships until the release suite passes on every desktop platform we
    // build for. Named up front so the build jobs below can depend on it.
    let gates: Vec<String> = Platform::iter()
        .filter(|platform| platform.is_desktop())
        .map(|platform| {
            let mut job = tests::test(tests::Suite::Release, platform);
            // A nightly that finds nothing new skips its builds; the gates must
            // skip with them rather than test a tree nobody will ship.
            if channel == Channel::Nightly {
                job.job = job
                    .job
                    .add_needs("prepare")
                    .cond(Expression::new(SHOULD_BUILD));
            }

            let id = job.id.clone();
            jobs.push(job);
            id
        })
        .collect();

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

    // Builds wait on the test gates, and on nightly also on the planner's
    // verdict. The gates and the planner wait on nothing.
    let scaffolding: Vec<&str> = ["prepare"]
        .into_iter()
        .chain(gates.iter().map(String::as_str))
        .collect();
    jobs = jobs
        .into_iter()
        .map(|named| {
            if scaffolding.contains(&named.id.as_str()) {
                return named;
            }

            let job = gates
                .iter()
                .fold(named.job, |job, gate| job.add_needs(gate.clone()));
            let job = match channel {
                Channel::Nightly => job.add_needs("prepare"),
                _ => job,
            };

            NamedJob { job, ..named }
        })
        .collect();

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
