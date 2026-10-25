//! Builds a channel's release workflow — one shape for all three, differing
//! only in flavor strings, tag pattern, and whether iOS is in scope.

use super::{TestSuite, WorkflowInput};
use super::{WorkflowContext, helpers, jobs};
use crate::{Channel, Platform};
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
    let prepare = channel.is_nightly().then(|| jobs::prepare(context));

    // Nothing ships until the release suite passes on every desktop platform.
    let tests: Vec<NamedJob> = Platform::iter()
        .filter(|platform| platform.is_desktop())
        .map(|platform| {
            // A nightly that finds nothing new skips its builds, so its gates
            // skip with them.
            jobs::test(TestSuite::Release, platform, context).map(|job| {
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
    let create = jobs::create_release(channel, prepare.as_ref(), context).map(|job| {
        gates
            .iter()
            .fold(job, |job, gate| job.add_needs(gate.clone()))
    });

    let platforms =
        WorkflowInput::choices("platforms", helpers::dispatch_platforms(), "All".into())
            .description("Platforms to build");

    // Defaulted to empty, not unset: an input with no default is `required`,
    // and blank is how a build-only run is asked for.
    let tag = WorkflowInput::string("tag", Some(String::new()))
        .description("Release tag (leave empty for build-only, no release)");

    // Only the tag-driven channels can hit the contradiction this guards:
    // nightly takes no `tag` dispatch input, since it mints its own.
    let guard = (!channel.is_nightly()).then(|| jobs::guard_dispatch(&tag, &platforms));

    // Builds wait on the test gates, the draft, and the guard; `build` wires
    // their planner dependency.
    let builds: Vec<NamedJob> = Platform::iter()
        .map(|platform| {
            jobs::build(channel, context, platform, prepare.as_ref()).map(|job| {
                let job = gates
                    .iter()
                    .fold(job, |job, gate| job.add_needs(gate.clone()))
                    .add_needs(create.name.clone());
                match &guard {
                    Some(guard) => job.add_needs(guard.name.clone()),
                    None => job,
                }
            })
        })
        .collect();

    // Uploads, publishes, and submits once every required target has
    // recorded in.
    let publish = jobs::publish(channel, prepare.as_ref()).map(|job| {
        builds
            .iter()
            .fold(job, |job, build| job.add_needs(build.name.clone()))
            .add_needs(create.name.clone())
    });

    // Cleans up a run that failed or fell short, on the same `needs` as the
    // publish it mirrors, and on the publish itself for its point of no
    // return.
    let withdraw = jobs::withdraw_release(channel, prepare.as_ref()).map(|job| {
        builds
            .iter()
            .fold(job, |job, build| job.add_needs(build.name.clone()))
            .add_needs(create.name.clone())
            .add_needs(publish.name.clone())
    });

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
                .chain(guard)
                .chain(builds)
                .chain([publish])
                .chain([withdraw])
                .fold(workflow, |workflow, named| {
                    workflow.add_job(named.name, named.job)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preview and stable take a `tag` dispatch input, so a subset
    /// `platforms` dispatch alongside it is a contradiction the guard job
    /// must catch before any build starts. Nightly takes no `tag` input, so
    /// the contradiction cannot arise there and the guard is absent.
    #[test]
    fn every_build_waits_on_the_dispatch_guard_where_one_exists() {
        let context = WorkflowContext::fixture();
        for channel in [Channel::Preview, Channel::Stable] {
            let workflow = workflow(channel, &context);
            let value = serde_json::to_value(&workflow).expect("workflow serializes");
            let jobs = value["jobs"].as_object().expect("jobs");

            assert!(
                jobs.contains_key("guard-dispatch"),
                "{channel}: no guard-dispatch job"
            );

            for platform in Platform::iter() {
                let name = format!("build-{}", platform.slug());
                let needs = jobs[&name]["needs"]
                    .as_array()
                    .unwrap_or_else(|| panic!("{channel}/{name}: no needs"));
                assert!(
                    needs.iter().any(|need| need == "guard-dispatch"),
                    "{channel}/{name}: does not need guard-dispatch"
                );
            }
        }

        let nightly = workflow(Channel::Nightly, &context);
        let value = serde_json::to_value(&nightly).expect("workflow serializes");
        assert!(
            !value["jobs"]
                .as_object()
                .expect("jobs")
                .contains_key("guard-dispatch"),
            "nightly takes no tag dispatch input, so the contradiction cannot arise"
        );
    }

    /// The job writing the manifest is the same `publish-release` job, which
    /// `needs` every build — so the manifest can never be written before
    /// every required target has reported its success.
    #[test]
    fn the_manifest_write_depends_on_every_build_job() {
        let context = WorkflowContext::fixture();
        for channel in [Channel::Nightly, Channel::Preview, Channel::Stable] {
            let workflow = workflow(channel, &context);
            let value = serde_json::to_value(&workflow).expect("workflow serializes");
            let jobs = value["jobs"].as_object().expect("jobs");

            let (job_name, job) = jobs
                .iter()
                .find(|(_, job)| {
                    job["steps"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|step| step["id"] == "write-manifest")
                })
                .unwrap_or_else(|| panic!("{channel}: no job writes the manifest"));
            assert_eq!(job_name, jobs::PUBLISH_JOB, "{channel}");
            assert!(
                !jobs.contains_key("publish-release"),
                "{channel}: the publishing job replaces publish-release"
            );

            let needs: Vec<&str> = job["needs"]
                .as_array()
                .unwrap_or_else(|| panic!("{channel}/{job_name}: no needs"))
                .iter()
                .map(|need| need.as_str().unwrap())
                .collect();

            for platform in Platform::iter() {
                let build_name = format!("build-{}", platform.slug());
                assert!(
                    needs.contains(&build_name.as_str()),
                    "{channel}/{job_name}: does not need {build_name}"
                );
            }
        }
    }

    /// The withdraw job reads every build's recorded target the same way
    /// the publish job does — to know whether there was a shortfall — so
    /// it must wait on the same set.
    #[test]
    fn the_withdraw_depends_on_every_build_job() {
        let context = WorkflowContext::fixture();
        for channel in [Channel::Nightly, Channel::Preview, Channel::Stable] {
            let workflow = workflow(channel, &context);
            let value = serde_json::to_value(&workflow).expect("workflow serializes");
            let jobs = value["jobs"].as_object().expect("jobs");

            let needs: Vec<&str> = jobs["withdraw-release"]["needs"]
                .as_array()
                .unwrap_or_else(|| panic!("{channel}: withdraw-release has no needs"))
                .iter()
                .map(|need| need.as_str().unwrap())
                .collect();

            for platform in Platform::iter() {
                let build_name = format!("build-{}", platform.slug());
                assert!(
                    needs.contains(&build_name.as_str()),
                    "{channel}: withdraw-release does not need {build_name}"
                );
            }
            assert!(
                needs.contains(&"create-release"),
                "{channel}: withdraw-release does not need create-release"
            );
            assert!(
                needs.contains(&jobs::PUBLISH_JOB),
                "{channel}: withdraw-release cannot read the point of no return"
            );
        }
    }
}
