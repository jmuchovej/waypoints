// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

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
    // Built once and lent to everything that depends on it.
    let prepare = jobs::prepare(channel, context);

    // Nothing ships until the release suite passes on every desktop platform.
    let tests: Vec<NamedJob> = Platform::iter()
        .filter(|platform| platform.is_desktop())
        .map(|platform| {
            // A run that builds nothing — a quiet nightly, or a dry run asked
            // not to build — skips its gates with its builds.
            jobs::test(TestSuite::Release, platform, context).map(|job| {
                let expr = format!("{} == 'true'", prepare.should_build.expr());
                job.add_needs(prepare.name().to_owned())
                    .cond(Expression::new(expr))
            })
        })
        .collect();

    // Named up front so every build can wait on the whole set.
    let gates: Vec<String> = tests.iter().map(|named| named.name.clone()).collect();

    // The draft the builds upload into — behind the same gates, so a failed
    // suite never drafts a release.
    let create = jobs::create_release(channel, &prepare, context).map(|job| {
        gates
            .iter()
            .fold(job, |job, gate| job.add_needs(gate.clone()))
    });

    let platforms =
        WorkflowInput::choices("platforms", helpers::dispatch_platforms(), "All".into())
            .description("Platforms to build");
    let build = WorkflowInput::bool("build", Some(true))
        .description("Run the test and build stages; off, the run only decides and reports");

    // Defaulted to empty, not unset: an input with no default is `required`,
    // and a dry run may leave it blank to take the manifest's version.
    let tag = WorkflowInput::string("tag", Some(String::new())).description(
        "The existing tag to publish; a dry run left blank uses the manifest's version",
    );
    // Every dispatch rehearses unless it says otherwise, so a forgotten flag
    // never re-publishes. Nightly has no such input: only its schedule
    // publishes, so its every dispatch is dry.
    let dry_run = WorkflowInput::bool("dry_run", Some(true)).description(
        "Rehearse: decide and build as asked, write only to the run's artifacts, publish nothing",
    );

    // Only the tag-driven channels can hit the contradiction this guards:
    // nightly takes no `tag` dispatch input, since it mints its own.
    let guard = (!channel.is_nightly()).then(|| jobs::guard_dispatch(&tag, &platforms, &dry_run));

    // Builds wait on the test gates, the draft, and the guard; `build` wires
    // their planner dependency.
    let builds: Vec<NamedJob> = Platform::iter()
        .map(|platform| {
            jobs::build(channel, context, platform, &prepare).map(|job| {
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
    let publish = jobs::publish(channel, &prepare).map(|job| {
        builds
            .iter()
            .fold(job, |job, build| job.add_needs(build.name.clone()))
            .add_needs(create.name.clone())
    });

    // Cleans up a run that failed or fell short, on the same `needs` as the
    // publish it mirrors, and on the publish itself for its point of no
    // return.
    let withdraw = jobs::withdraw_release(channel, &prepare).map(|job| {
        builds
            .iter()
            .fold(job, |job, build| job.add_needs(build.name.clone()))
            .add_needs(create.name.clone())
            .add_needs(publish.name.clone())
    });

    let dispatch = WorkflowDispatch::default()
        .inputs([
            (platforms.name.into(), platforms.input()),
            (build.name.into(), build.input()),
        ])
        .when(!channel.is_nightly(), |mut dispatch| {
            dispatch.inputs.insert(tag.name.into(), tag.input());
            dispatch.inputs.insert(dry_run.name.into(), dry_run.input());
            dispatch
        });

    let prepare: NamedJob = prepare.into();
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
        .when(channel.is_nightly(), |workflow| {
            workflow.concurrency(helpers::serialize("nightly"))
        })
        .add_job(prepare.name, prepare.job)
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
    use serde_json::Value;

    const CHANNELS: [Channel; 3] = [Channel::Nightly, Channel::Preview, Channel::Stable];

    fn rendered(channel: Channel) -> Value {
        let workflow = workflow(channel, &WorkflowContext::fixture());
        serde_json::to_value(&workflow).expect("workflow serializes")
    }

    fn needs(job: &Value) -> Vec<&str> {
        job["needs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|need| need.as_str().unwrap())
            .collect()
    }

    /// Every channel decides before it does anything: the planner job is
    /// there, and every job but the guard waits on it.
    #[test]
    fn every_channel_plans_first() {
        for channel in CHANNELS {
            let value = rendered(channel);
            let jobs = value["jobs"].as_object().expect("jobs");
            assert!(jobs.contains_key("prepare"), "{channel}: no prepare job");
            for (name, job) in jobs {
                if name == "prepare" || name == "guard-dispatch" {
                    continue;
                }
                assert!(
                    needs(job).contains(&"prepare"),
                    "{channel}/{name}: does not need prepare"
                );
            }
        }
    }

    /// The dispatch inputs, and their defaults, per design D4: nightly takes
    /// `build` and `platforms` and is always dry; the others add `tag` and
    /// `dry_run`, defaulting to a rehearsal.
    #[test]
    fn each_workflow_offers_its_dispatch_inputs() {
        for channel in CHANNELS {
            let value = rendered(channel);
            let inputs = value["on"]["workflow_dispatch"]["inputs"]
                .as_object()
                .expect("inputs");
            let mut names: Vec<&str> = inputs.keys().map(String::as_str).collect();
            names.sort_unstable();
            let expected: &[&str] = if channel.is_nightly() {
                &["build", "platforms"]
            } else {
                &["build", "dry_run", "platforms", "tag"]
            };
            assert_eq!(names, expected, "{channel}");

            let default = |name: &str| inputs[name]["default"].to_string().replace('"', "");
            assert_eq!(default("build"), "true", "{channel}");
            assert_eq!(default("platforms"), "All", "{channel}");
            if !channel.is_nightly() {
                assert_eq!(default("dry_run"), "true", "{channel}");
                assert_eq!(default("tag"), "", "{channel}");
            }
        }
    }

    /// The planner's flags come from the dispatch, with the event guard
    /// leading every expression: on a push `inputs` is empty, and `!null`
    /// is true.
    #[test]
    fn the_planner_takes_its_flags_from_the_dispatch() {
        for channel in CHANNELS {
            let value = rendered(channel);
            let steps = value["jobs"]["prepare"]["steps"].as_array().expect("steps");
            let run = steps
                .iter()
                .find(|step| step["id"] == "publish-plan")
                .and_then(|step| step["run"].as_str())
                .unwrap_or_else(|| panic!("{channel}: no publish-plan step"));
            assert!(
                run.starts_with(&format!("cargo xtask publish-plan {}", channel.flavor())),
                "{channel}: {run}"
            );
            let dry = if channel.is_nightly() {
                "${{ github.event_name == 'workflow_dispatch' && '--dry-run' || '' }}"
            } else {
                "${{ github.event_name == 'workflow_dispatch' && inputs.dry_run && '--dry-run' || '' }}"
            };
            assert!(run.contains(dry), "{channel}: {run}");
            assert!(
                run.contains(
                    "${{ github.event_name == 'workflow_dispatch' && !inputs.build && '--no-build' || '' }}"
                ),
                "{channel}: {run}"
            );
            assert_eq!(
                run.contains(r#"--tag "$TAG""#),
                !channel.is_nightly(),
                "{channel}: {run}"
            );
        }
    }

    /// Preview and stable take a `tag` dispatch input, so a subset
    /// `platforms` dispatch alongside it is a contradiction the guard job
    /// must catch before any build starts — on a real run; a dry run may
    /// build a subset. Nightly takes no `tag` input, so the contradiction
    /// cannot arise there and the guard is absent.
    #[test]
    fn every_build_waits_on_the_dispatch_guard_where_one_exists() {
        for channel in [Channel::Preview, Channel::Stable] {
            let value = rendered(channel);
            let jobs = value["jobs"].as_object().expect("jobs");

            let guard = jobs
                .get("guard-dispatch")
                .unwrap_or_else(|| panic!("{channel}: no guard-dispatch job"));
            let cond = guard["if"].as_str().expect("if");
            assert!(cond.contains("!inputs.dry_run"), "{channel}: {cond}");

            for platform in Platform::iter() {
                let name = format!("build-{}", platform.slug());
                assert!(
                    needs(&jobs[&name]).contains(&"guard-dispatch"),
                    "{channel}/{name}: does not need guard-dispatch"
                );
            }
        }

        let nightly = rendered(Channel::Nightly);
        assert!(
            !nightly["jobs"]
                .as_object()
                .expect("jobs")
                .contains_key("guard-dispatch"),
            "nightly takes no tag dispatch input, so the contradiction cannot arise"
        );
    }

    /// The job writing the manifest is the same `publish` job, which `needs`
    /// every build — so the manifest can never be written before every
    /// required target has reported its success.
    #[test]
    fn the_manifest_write_depends_on_every_build_job() {
        for channel in CHANNELS {
            let value = rendered(channel);
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

            let needs = needs(job);
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
        for channel in CHANNELS {
            let value = rendered(channel);
            let jobs = value["jobs"].as_object().expect("jobs");
            let needs = needs(&jobs["withdraw-release"]);
            assert!(
                !needs.is_empty(),
                "{channel}: withdraw-release has no needs"
            );

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
