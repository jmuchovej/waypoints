//! Shared expressions, secret handles, and concurrency groups.
//! Trigger plumbing shared by workflow modules.

use gh_workflow::{Concurrency, Environment, Permissions, Run, Step, Workflow};

macro_rules! secret {
    ($name:ident) => {
        pub const $name: &str = concat!("${{ secrets.", stringify!($name), " }}");
    };
}
macro_rules! env {
    ($name:ident) => {
        pub const $name: &str = concat!("${{ env.", stringify!($name), " }}");
    };
}

secret!(GITHUB_TOKEN);
secret!(OP_SERVICE_ACCOUNT_TOKEN);
env!(ANDROID_HOME);
env!(WP_BOTSWAIN_APP_ID);
env!(WP_BOTSWAIN_PRIVATE_KEY);

/// A secretspec profile CI resolves, and the GitHub environment whose secrets
/// hold the vault credential for it.
///
/// The environment is the reach boundary: a job declares one, reads
/// `OP_SERVICE_ACCOUNT_TOKEN` from it, and nothing at repository scope answers
/// to that name. Both environments carry the same service account today; when
/// they diverge, nothing here changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Signing material, resolved under a platform scope by the build jobs.
    Release,
    /// The wp-botswain App credentials a run authenticates as.
    Automation,
}

impl Profile {
    /// The `--profile` secretspec is given.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Release => "release",
            Profile::Automation => "automation",
        }
    }

    /// The GitHub environment a job resolving this profile declares.
    pub fn environment(self) -> Environment {
        let name = match self {
            Profile::Release => "App Release",
            Profile::Automation => "CI",
        };
        Environment {
            name: name.to_owned(),
            url: None,
        }
    }
}

/// The manifest alias for the production vault. Set on the resolving step and
/// nowhere wider: devenv's shell injects `SECRETSPEC_PROVIDER=env`, which would
/// otherwise win inside any step that enters it, and a workflow-level value is
/// what used to force every shell entry — secret or not — onto a store the
/// runner could not read.
const PROVIDER: (&str, &str) = ("SECRETSPEC_PROVIDER", "op");

/// The vault credential, read from the job's declared environment.
const TOKEN: (&str, &str) = ("OP_SERVICE_ACCOUNT_TOKEN", OP_SERVICE_ACCOUNT_TOKEN);

/// Gives a step what it needs to resolve secrets from the vault. Every step
/// that runs `secretspec` against a CI-facing profile goes through here, so
/// the provider and the credential cannot be set one without the other.
pub fn resolving(step: Step<Run>) -> Step<Run> {
    step.add_env(PROVIDER).add_env(TOKEN)
}

/// Pins JavaScript actions to Node 24; the runner otherwise falls back to its
/// own default, which they fail to start under.
pub const FORCE_NODE24: (&str, &str) = ("FORCE_JAVASCRIPT_ACTIONS_TO_NODE24", "true");

/// One run per ref, cancelling any already in flight. For checks only — a
/// cancelled release leaves published artifacts half-written.
pub fn one_run_per_ref(prefix: &str) -> Concurrency {
    Concurrency::default()
        .group(format!("{prefix}-${{{{ github.ref }}}}"))
        .cancel_in_progress(true)
}

/// One run at a time, queued rather than cancelled.
pub fn serialize(group: &str) -> Concurrency {
    Concurrency::default()
        .group(group.to_owned())
        .cancel_in_progress(false)
}

pub fn named_workflow(name: impl ToString) -> Workflow {
    Workflow::new(name)
        // Blank, so nothing inherits by accident: every job states its own.
        .permissions(Permissions::default())
        .add_env(FORCE_NODE24)
}
