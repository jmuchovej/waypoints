//! Builds a channel's release workflow — one shape for all three, differing
//! only in flavor strings, tag pattern, and whether iOS is in scope.

use super::{Channel, Platform, TestSuite, WorkflowContext, WorkflowInput, helpers, jobs};
use gh_workflow::{
    Event, Expression, FluentBuilder, NamedJob, Push, Schedule, Workflow, WorkflowDispatch,
};
use heck::ToTitleCase;
use strum::IntoEnumIterator;

/// Nightly runs at 07:00 UTC (~03:00 US Eastern). `schedule` only fires on the
/// default branch, which is what a nightly should track.
const NIGHTLY_CRON: &str = "0 7 * * *";

pub fn workflow(channel: Channel, context: &WorkflowContext) -> Workflow {
    // Built once and lent to everything that depends on it; only nightly has
    // a planner.
    let prepare = channel
        .is_nightly()
        .then(|| jobs::prepare(&context.formatters));

    // Nothing ships until the release suite passes on every desktop platform.
    let tests: Vec<NamedJob> = Platform::iter()
        .filter(|platform| platform.is_desktop())
        .map(|platform| {
            // A nightly that finds nothing new skips its builds, so its gates
            // skip with them.
            jobs::test(TestSuite::Release, platform, &context.formatters).map(|job| {
                job.when_some(prepare.as_ref(), |job, prepare| {
                    let expr = format!("{} == 'true'", prepare.should_build.expr());
                    job.add_needs(prepare.name().to_owned())
                        .cond(Expression::new(expr))
                })
            })
        })
        .collect();

    // Named up front so every build can wait on the whole set.
    let gates: Vec<String> = tests.iter().map(|named| named.name.clone()).collect();

    // The draft the builds upload into — behind the same gates, so a failed
    // suite never drafts a release.
    let create = jobs::create_release(channel, prepare.as_ref()).map(|job| {
        gates
            .iter()
            .fold(job, |job, gate| job.add_needs(gate.clone()))
    });

    // Builds wait on the test gates and the draft; `build` wires their
    // planner dependency.
    let builds: Vec<NamedJob> = Platform::iter()
        .map(|platform| {
            jobs::build(channel, context, platform, prepare.as_ref()).map(|job| {
                gates
                    .iter()
                    .fold(job, |job, gate| job.add_needs(gate.clone()))
                    .add_needs(create.name.clone())
            })
        })
        .collect();

    // Flips the draft public once every build has reported in.
    let publish = jobs::publish_release(prepare.as_ref()).map(|job| {
        builds
            .iter()
            .fold(job, |job, build| job.add_needs(build.name.clone()))
            .add_needs(create.name.clone())
    });

    let platforms = WorkflowInput::choices("platforms", channel.dispatch_platforms(), "All".into())
        .description("Platforms to build");

    // Defaulted to empty, not unset: an input with no default is `required`,
    // and blank is how a build-only run is asked for.
    let tag = WorkflowInput::string("tag", Some(String::new()))
        .description("Release tag (leave empty for build-only, no release)");

    // Nightly mints its own tag, so only the tag-driven channels take one.
    let dispatch = WorkflowDispatch::default()
        .inputs([(platforms.name.into(), platforms.input())])
        .when(!channel.is_nightly(), |mut dispatch| {
            dispatch.inputs.insert(tag.name.into(), tag.input());
            dispatch
        });

    helpers::named_workflow(channel.flavor().to_title_case())
        .on(Event::default()
            .workflow_dispatch(dispatch)
            .when_some(channel.tag_pattern(), |event, pattern| {
                event.push(Push::default().add_tag(pattern))
            })
            .when(channel.is_nightly(), |event| {
                event.schedule(vec![Schedule::new(NIGHTLY_CRON)])
            }))
        // A scheduled run and a manual or tag run must never bundle at once,
        // and cancelling one mid-publish would strand release artifacts.
        .when_some(prepare, |workflow, prepare| {
            let prepare: NamedJob = prepare.into();
            workflow
                .concurrency(helpers::serialize("nightly"))
                .add_job(prepare.name, prepare.job)
        })
        .map(|workflow| {
            tests
                .into_iter()
                .chain([create])
                .chain(builds)
                .chain([publish])
                .fold(workflow, |workflow, named| {
                    workflow.add_job(named.name, named.job)
                })
        })
}
