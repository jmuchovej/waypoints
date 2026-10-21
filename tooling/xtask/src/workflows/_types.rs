//! Types the workflow modules are assembled from: inputs, outputs, and the
//! names jobs and artifacts agree on.

use crate::{Channel, Platform};
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

/// The `upload-artifact` name for one build: `waypoints-{platform}-{channel}`.
///
/// A newtype rather than a plain `(Channel, Platform)`: the orphan rule counts
/// tuples as foreign, so `Display` cannot be implemented on one.
#[derive(Display)]
#[display("waypoints-{}-{}", _1.slug(), _0)]
pub struct Artifact(pub Channel, pub Platform);

/// The nightly planner, together with the outputs its dependents read back.
///
/// Built once and lent to the jobs that depend on it. Each output already
/// carries the job name, so nothing downstream respells either that or the
/// output keys — and a typo becomes a missing field rather than an expression
/// that silently resolves to an empty string.
pub struct PrepareJob {
    job: NamedJob,
    pub tag: JobOutput,
    /// Whether this run itself created `tag`'s ref. True only on the
    /// publish path — the withdraw job reads it back to decide whether it
    /// may delete the tag.
    pub tag_minted: JobOutput,
    pub should_build: JobOutput,
    pub version: JobOutput,
    pub version_code: JobOutput,
    pub bundle_version: JobOutput,
}

impl PrepareJob {
    /// Outputs first: each is derived from `job`, so they have to be built
    /// before it is moved in.
    pub fn new(
        tag: JobOutput,
        tag_minted: JobOutput,
        should_build: JobOutput,
        version: JobOutput,
        version_code: JobOutput,
        bundle_version: JobOutput,
        job: NamedJob,
    ) -> Self {
        Self {
            job,
            tag,
            tag_minted,
            should_build,
            version,
            version_code,
            bundle_version,
        }
    }

    /// The job id, for wiring `needs:` without consuming the job.
    pub fn name(&self) -> &str {
        &self.job.name
    }
}

impl From<PrepareJob> for NamedJob {
    fn from(prepare: PrepareJob) -> Self {
        prepare.job
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

    pub fn call_input(&self) -> WorkflowCallInput {
        WorkflowCallInput {
            description: self.name.to_owned(),
            required: self.default.is_none(),
            input_type: self.input_type.to_owned(),
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
