// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Types the workflow modules are assembled from: inputs, outputs, and the
//! names jobs and artifacts agree on.

use crate::Platform;
use derive_more::Display;
use gh_workflow::{
    JobOutput, NamedJob, WorkflowCallInput, WorkflowCallSecret, WorkflowDispatchInput,
};

/// Which test suite to run.
#[derive(Clone, Copy)]
pub enum TestSuite {
    /// Everything the shipped artifacts are built from. Gates a release.
    Release,
    /// Only what the change can break.
    Changed,
}

impl TestSuite {
    pub(crate) fn flag(self) -> &'static str {
        match self {
            TestSuite::Release => " --release",
            TestSuite::Changed => "",
        }
    }

    pub(crate) fn describe(self) -> &'static str {
        match self {
            TestSuite::Release => "release",
            TestSuite::Changed => "affected",
        }
    }
}

/// The `upload-artifact` name for one build: `waypoints-{platform}-{channel}`,
/// the channel a name or an expression over the describing job's outputs.
///
/// A newtype rather than a plain tuple: the orphan rule counts tuples as
/// foreign, so `Display` cannot be implemented on one.
#[derive(Display)]
#[display("waypoints-{}-{}", _1.slug(), _0)]
pub struct Artifact(pub String, pub Platform);

/// The builder's first job, together with the outputs its dependents read
/// back: the build's identity, as `release describe` read it from the ref.
pub struct DescribeJob {
    job: NamedJob,
    pub tag: JobOutput,
    pub version: JobOutput,
    pub channel: JobOutput,
    pub flavor: JobOutput,
    pub version_code: JobOutput,
    pub bundle_version: JobOutput,
}

impl DescribeJob {
    /// Outputs first: each is derived from `job`, so they have to be built
    /// before it is moved in.
    #[allow(
        clippy::too_many_arguments,
        reason = "one parameter per output, the fields the struct carries"
    )]
    pub fn new(
        tag: JobOutput,
        version: JobOutput,
        channel: JobOutput,
        flavor: JobOutput,
        version_code: JobOutput,
        bundle_version: JobOutput,
        job: NamedJob,
    ) -> Self {
        Self {
            job,
            tag,
            version,
            channel,
            flavor,
            version_code,
            bundle_version,
        }
    }

    /// The job id, for wiring `needs:` without consuming the job.
    pub fn name(&self) -> &str {
        &self.job.name
    }
}

impl From<DescribeJob> for NamedJob {
    fn from(describe: DescribeJob) -> Self {
        describe.job
    }
}

/// The release run's guard, together with the one flag it reads off the
/// event: whether the run rehearses.
pub struct GuardJob {
    job: NamedJob,
    pub dry_run: JobOutput,
}

impl GuardJob {
    pub fn new(dry_run: JobOutput, job: NamedJob) -> Self {
        Self { job, dry_run }
    }

    /// The job id, for wiring `needs:` without consuming the job.
    pub fn name(&self) -> &str {
        &self.job.name
    }
}

impl From<GuardJob> for NamedJob {
    fn from(guard: GuardJob) -> Self {
        guard.job
    }
}

/// What the release workflow's jobs read of the run: the identity the
/// builder hands back through the job that called it, the guard's rehearsal
/// flag, and the two job ids to wait on.
pub struct ReleaseRun {
    pub guard: String,
    pub build: String,
    pub tag: JobOutput,
    pub channel: JobOutput,
    pub flavor: JobOutput,
    pub dry_run: JobOutput,
}

impl ReleaseRun {
    pub fn new(guard: &GuardJob, build: &NamedJob) -> Self {
        let output = |name| JobOutput::new(build.name.clone(), name);
        Self {
            guard: guard.name().to_owned(),
            build: build.name.clone(),
            tag: output("tag"),
            channel: output("channel"),
            flavor: output("flavor"),
            dry_run: guard.dry_run.clone(),
        }
    }

    /// The builder's output carrying `platform`'s recorded target, as an
    /// expression.
    pub fn target(&self, platform: Platform) -> String {
        format!("needs.{}.outputs.{}", self.build, platform.slug())
    }
}

#[derive(Clone)]
pub struct WorkflowInput {
    pub input_type: &'static str,
    pub name: &'static str,
    pub default: Option<String>,
    pub description: Option<String>,
    pub options: Vec<String>,
}

impl WorkflowInput {
    pub fn string(name: &'static str, default: Option<String>) -> Self {
        Self {
            input_type: "string",
            name,
            default,
            description: None,
            options: vec![],
        }
    }

    pub fn bool(name: &'static str, default: Option<bool>) -> Self {
        Self {
            input_type: "boolean",
            name,
            default: default.as_ref().map(ToString::to_string),
            description: None,
            options: vec![],
        }
    }

    pub fn choices(
        name: &'static str,
        choices: Vec<impl ToString>,
        default: Option<impl ToString>,
    ) -> Self {
        Self {
            input_type: "choice",
            name,
            default: default.as_ref().map(ToString::to_string),
            description: None,
            options: choices.iter().map(ToString::to_string).collect(),
        }
    }

    pub fn description(mut self, description: impl ToString) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn input(&self) -> WorkflowDispatchInput {
        WorkflowDispatchInput {
            description: self
                .description
                .clone()
                .unwrap_or_else(|| self.name.to_owned()),
            required: self.default.is_none(),
            input_type: self.input_type.to_owned(),
            default: self.default.clone(),
            options: self.options.clone(),
        }
    }

    /// The same input as a called workflow takes it. A call has no choice
    /// type, so a choice is a string there, and the caller's dispatch offers
    /// the list.
    pub fn call_input(&self) -> WorkflowCallInput {
        WorkflowCallInput {
            description: self
                .description
                .clone()
                .unwrap_or_else(|| self.name.to_owned()),
            required: self.default.is_none(),
            input_type: if self.input_type == "choice" {
                "string".to_owned()
            } else {
                self.input_type.to_owned()
            },
            default: self.default.clone(),
        }
    }

    pub fn expr(&self) -> String {
        format!("inputs.{}", self.name)
    }
}

impl std::fmt::Display for WorkflowInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${{{{ {} }}}}", self.expr())
    }
}

impl serde::Serialize for WorkflowInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<WorkflowInput> for (String, WorkflowDispatchInput) {
    fn from(value: WorkflowInput) -> Self {
        (value.name.into(), value.input())
    }
}

#[derive(Clone)]
#[allow(dead_code, reason = "for workflows that take `secrets:` inputs")]
pub struct WorkflowSecret {
    pub name: &'static str,
    description: String,
    required: bool,
}

#[allow(dead_code, reason = "for workflows that take `secrets:` inputs")]
impl WorkflowSecret {
    pub fn new(name: &'static str, description: impl ToString) -> Self {
        Self {
            name,
            description: description.to_string(),
            required: true,
        }
    }

    pub fn secret_configuration(&self) -> WorkflowCallSecret {
        WorkflowCallSecret {
            description: self.description.clone(),
            required: self.required,
        }
    }
}

impl std::fmt::Display for WorkflowSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${{{{ secrets.{} }}}}", self.name)
    }
}

impl serde::Serialize for WorkflowSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
