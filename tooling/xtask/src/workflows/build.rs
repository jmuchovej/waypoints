// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The builder: tests and builds the tree a ref names, and hands the
//! distributables to its caller as run artifacts. `on: workflow_call` alone,
//! so it runs inside a caller's run and knows nothing of why it was called —
//! no election, no publishing, no store, forge, or App credential. Its first
//! job reads the ref into the build's identity with `release describe`; the
//! release test gates and the five builds follow, each checking the ref out
//! and stamping what `describe` read.

use super::{TestSuite, WorkflowContext, WorkflowInput, helpers, jobs, jobs::BuildInputs};
use crate::Platform;
use gh_workflow::{Event, FluentBuilder, NamedJob, Workflow, WorkflowCall, WorkflowCallOutput};
use strum::IntoEnumIterator;

/// The outputs every caller reads back, by name, from the describing job.
const IDENTITY: [(&str, &str); 4] = [
    (
        "tag",
        "The tag the build is for: the ref, or the one its tree would be tagged as",
    ),
    ("version", "The version the build stamps"),
    ("channel", "The channel the version names"),
    ("flavor", "The flavor the build was made under"),
];

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let reference = WorkflowInput::string("ref", None).description(
        "The ref to test and build: a tag, or any commit for the build its tree would make",
    );
    // A string rather than a choice: a called workflow's inputs have no
    // choice type, so the caller's dispatch offers the list.
    let platforms = WorkflowInput::string("platforms", Some("All".to_owned()))
        .description("The platforms to build: All, Desktop, Mobile, or one platform");

    let describe = jobs::describe(context, &reference);

    // Nothing is built until the release suite passes on every desktop
    // platform, on the tree the ref names.
    let gates: Vec<NamedJob> = Platform::iter()
        .filter(|platform| platform.is_desktop())
        .map(|platform| {
            jobs::test(
                TestSuite::Release,
                platform,
                context,
                Some(&reference.to_string()),
            )
            .map(|job| job.add_needs(describe.name().to_owned()))
        })
        .collect();
    let gate_names: Vec<String> = gates.iter().map(|named| named.name.clone()).collect();

    let builds: Vec<NamedJob> = Platform::iter()
        .map(|platform| {
            let inputs = BuildInputs::described(&describe, &reference, platform);
            jobs::build(context, platform, &inputs).map(|job| {
                gate_names
                    .iter()
                    .fold(job, |job, gate| job.add_needs(gate.clone()))
            })
        })
        .collect();

    let mut call = WorkflowCall::default()
        .add_input(reference.name, reference.call_input())
        .add_input(platforms.name, platforms.call_input());
    for (name, description) in IDENTITY {
        call.outputs.insert(
            name.to_owned(),
            WorkflowCallOutput {
                description: description.to_owned(),
                value: format!("${{{{ jobs.{}.outputs.{name} }}}}", describe.name()),
            },
        );
    }
    // One output per platform, carrying its slug once its build recorded in:
    // what the caller counts against the channel's required set.
    for platform in Platform::iter() {
        call.outputs.insert(
            platform.slug(),
            WorkflowCallOutput {
                description: format!(
                    "`{}` once the {platform} build recorded in",
                    platform.slug()
                ),
                value: format!(
                    "${{{{ jobs.{}.outputs.target }}}}",
                    jobs::build_job_name(platform)
                ),
            },
        );
    }

    let describe: NamedJob = describe.into();
    helpers::named_workflow("build the distributables of a ref")
        .on(Event::default().workflow_call(call))
        .add_job(describe.name, describe.job)
        .map(|workflow| {
            gates
                .into_iter()
                .chain(builds)
                .fold(workflow, |workflow, named| {
                    workflow.add_job(named.name, named.job)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn rendered() -> Value {
        serde_json::to_value(workflow(&WorkflowContext::fixture())).expect("workflow serializes")
    }

    fn steps(job: &Value) -> Vec<&Value> {
        job["steps"].as_array().into_iter().flatten().collect()
    }

    fn step<'a>(job: &'a Value, id: &str) -> &'a Value {
        steps(job)
            .into_iter()
            .find(|step| step["id"] == id)
            .unwrap_or_else(|| panic!("no step `{id}`"))
    }

    /// The builder is called, never triggered, with the ref and the platforms
    /// it builds; it hands back the identity `describe` read and each target
    /// that recorded in, which is what its caller counts.
    #[test]
    fn the_builder_is_called_and_reports_its_identity_and_targets() {
        let value = rendered();
        let on = value["on"].as_object().expect("triggers");
        assert_eq!(on.keys().collect::<Vec<_>>(), ["workflow_call"]);

        let inputs = &on["workflow_call"]["inputs"];
        assert_eq!(inputs["ref"]["required"], true, "{inputs}");
        assert_eq!(inputs["ref"]["type"], "string");
        assert_eq!(inputs["platforms"]["default"], "All");
        assert!(inputs["platforms"].get("required").is_none(), "{inputs}");
        assert!(on["workflow_call"].get("secrets").is_none(), "{on:?}");

        let outputs = on["workflow_call"]["outputs"].as_object().expect("outputs");
        for (name, _) in IDENTITY {
            assert_eq!(
                outputs[name]["value"],
                format!("${{{{ jobs.describe.outputs.{name} }}}}")
            );
        }
        for platform in Platform::iter() {
            let slug = platform.slug();
            assert_eq!(
                outputs[&slug]["value"],
                format!("${{{{ jobs.build-{slug}.outputs.target }}}}")
            );
        }
        assert_eq!(outputs.len(), IDENTITY.len() + Platform::iter().count());
    }

    /// Scenario _The builder stamps from the ref_: every job checks the ref
    /// out, and the describing job and the builds take the history and the
    /// tags a tag resolves and `git describe` reads.
    #[test]
    fn every_builder_job_checks_out_the_ref() {
        let value = rendered();
        let jobs = value["jobs"].as_object().expect("jobs");
        assert!(jobs.len() > 1);
        for (name, job) in jobs {
            let checkout = step(job, "checkout");
            assert_eq!(checkout["with"]["ref"], "${{ inputs.ref }}", "{name}");
            if name == "describe" || name.starts_with("build-") {
                assert_eq!(checkout["with"]["fetch-depth"], "0", "{name}");
                assert_eq!(checkout["with"]["fetch-tags"], "true", "{name}");
            }
        }
        let describe = step(&value["jobs"]["describe"], "describe");
        assert_eq!(
            describe["run"],
            r#"cargo ci-task release describe --ref "$REF""#
        );
        assert_eq!(describe["env"]["REF"], "${{ inputs.ref }}");
    }

    /// The builds stamp what `describe` read, and nothing else reaches them
    /// from another job: the flavor, the version on the mobile platforms, the
    /// build code where the store has one, the tag the files are named for,
    /// and the channel the artifact is named for.
    #[test]
    fn the_builds_read_their_identity_from_describe() {
        let value = rendered();
        for platform in Platform::iter() {
            let slug = platform.slug();
            let job = &value["jobs"][format!("build-{slug}")];
            let needs: Vec<&str> = job["needs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|need| need.as_str().unwrap())
                .collect();
            assert!(needs.contains(&"describe"), "{slug}: {needs:?}");
            assert_eq!(
                job["if"],
                helpers::selected(platform),
                "{slug}: the gate is the caller's platforms alone"
            );

            let recipe = step(job, "build-recipe");
            assert_eq!(
                recipe["run"],
                format!(r#"just tauri::build {slug} "${{FLAVOR}}""#)
            );
            assert_eq!(
                recipe["env"]["FLAVOR"],
                "${{ needs.describe.outputs.flavor }}"
            );
            let version = recipe["env"].get(jobs::VERSION_ENV);
            let code = recipe["env"].get(jobs::BUILD_CODE_ENV);
            match platform {
                Platform::Android => {
                    assert_eq!(version.unwrap(), "${{ needs.describe.outputs.version }}");
                    assert_eq!(code.unwrap(), "${{ needs.describe.outputs.version_code }}");
                }
                Platform::iOS => {
                    assert_eq!(version.unwrap(), "${{ needs.describe.outputs.version }}");
                    assert_eq!(
                        code.unwrap(),
                        "${{ needs.describe.outputs.bundle_version }}"
                    );
                }
                _ => assert!(version.is_none() && code.is_none(), "{slug}"),
            }
            for step in steps(job) {
                for (key, value) in step["env"].as_object().into_iter().flatten() {
                    let text = value.as_str().unwrap_or("");
                    assert!(
                        !text.contains("needs.") || text.contains("needs.describe.outputs."),
                        "{slug}/{}: `{key}` reads another job: {text}",
                        step["name"]
                    );
                }
            }

            assert_eq!(
                step(job, "stage-release")["env"]["TAG"],
                "${{ needs.describe.outputs.tag }}",
                "{slug}"
            );
            assert_eq!(
                step(job, "upload-artifacts")["with"]["name"],
                format!("waypoints-{slug}-${{{{ needs.describe.outputs.channel }}}}")
            );
        }
    }

    /// Scenario _The builder publishes nothing_: no job declares a store
    /// scope, mints an App token, or names the store, the forge, or a
    /// submission. The vault credential it does hold is the cache's and the
    /// signing's, inside the recipe.
    #[test]
    fn the_builder_holds_no_publishing_credential_or_step() {
        let value = rendered();
        for (name, job) in value["jobs"].as_object().expect("jobs") {
            for step in steps(job) {
                let label = format!("{name}: {}", step["name"]);
                let uses = step["uses"].as_str().unwrap_or("");
                assert!(
                    !uses.starts_with("actions/create-github-app-token@"),
                    "{label}: mints an App token"
                );
                let run = step["run"].as_str().unwrap_or("");
                for forbidden in [
                    "release distributables upload",
                    "release distributables commit",
                    "release distributables withdraw",
                    "release record",
                    "release merge",
                    "release tag",
                    "release open-series",
                    "release undo",
                    "xfastlane",
                    "gh release",
                    "gh api",
                    "--profile release",
                ] {
                    assert!(!run.contains(forbidden), "{label}: `{forbidden}`");
                }
            }
        }
    }
}
