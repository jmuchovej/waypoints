//! Shared expressions, secret handles, and concurrency groups.
//! Trigger plumbing shared by workflow modules.

use crate::Platform;
use gh_workflow::{
    Concurrency, Environment as WorkflowEnvironment, Expression, JobOutput, Permissions, Run, Step,
    Workflow,
};

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

/// The R2 bucket every CI cache lives in — sccache's objects under `sccache/`,
/// directory snapshots under `dirs/`. Named for its one writer class rather
/// than its contents: only a job holding an environment's vault credential can
/// write to it, which is what lets a publishing job restore from it.
pub const CI_CACHE_BUCKET: &str = "ci-cache";

/// A secretspec profile CI resolves. Neither is ever resolved whole — see
/// [`Resolution`] for the scope that always narrows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Signing material, one scope per platform.
    Release,
    /// Every non-signing credential CI uses, one scope per consumer.
    Ci,
}

impl Profile {
    /// The `--profile` secretspec is given.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Release => "release",
            Profile::Ci => "ci",
        }
    }
}

/// One consumer's slice of the `ci` profile — the `[scopes.<name>]` in
/// `secretspec.toml` it resolves under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiScope {
    /// The build-cache store's endpoint and S3 key pair.
    Cache,
    /// The wp-botswain GitHub App a run authenticates as.
    Botswain,
}

impl CiScope {
    pub fn name(self) -> &'static str {
        match self {
            CiScope::Cache => "cache",
            CiScope::Botswain => "botswain",
        }
    }
}

/// What a step resolves: a CI-facing profile narrowed to one consumer's scope.
///
/// There is no unscoped variant on purpose. A resolution is the unit a job
/// receives, and the manifest's guarantee — one consumer, one set of secrets —
/// holds only while every resolution names its scope. Making the pair one value
/// is what keeps the generator from emitting `--profile ci` on its own.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The release profile under one platform's signing scope.
    Release(Platform),
    /// The ci profile under one consumer's scope.
    Ci(CiScope),
}

impl Resolution {
    pub fn profile(self) -> Profile {
        match self {
            Resolution::Release(_) => Profile::Release,
            Resolution::Ci(_) => Profile::Ci,
        }
    }

    /// The `--scope` secretspec is given.
    pub fn scope(self) -> String {
        match self {
            Resolution::Release(platform) => platform.slug(),
            Resolution::Ci(scope) => scope.name().to_owned(),
        }
    }
}

/// A GitHub environment a job declares for the work it does. The vault
/// credential is read from it, so the environment is the reach boundary: a job
/// declares one, reads `OP_SERVICE_ACCOUNT_TOKEN` from it, and nothing at
/// repository scope answers to that name.
///
/// Declared by the job, not derived from what it resolves — the same `ci`
/// profile is resolved by builds under `App Release` and by automation under
/// `CI`. Both environments carry the same service account today; when they
/// diverge, nothing here changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    /// Builds, signs, and publishes application artifacts.
    AppRelease,
    /// Authenticates as the automation identity.
    Ci,
}

impl Environment {
    pub fn name(self) -> &'static str {
        match self {
            Environment::AppRelease => "App Release",
            Environment::Ci => "CI",
        }
    }
}

impl From<Environment> for WorkflowEnvironment {
    fn from(environment: Environment) -> Self {
        WorkflowEnvironment {
            name: environment.name().to_owned(),
            url: None,
        }
    }
}

/// The manifest alias for the production vault. The profile defaults to it
/// already; it is set on the resolving step anyway so the generated YAML says
/// where that step reads from, and nowhere wider — a workflow-level value is
/// what used to force every shell entry, secret or not, onto a store the
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

/// The vault provider, for a command that names it with `--provider` rather
/// than reading it from the step environment.
pub const PROVIDER_NAME: &str = PROVIDER.1;

/// The credential alone, for a step whose command carries [`PROVIDER_NAME`]
/// itself. Needed under `devenv shell`: devenv resolves the shell's own
/// profile before the shell exists and honours `SECRETSPEC_PROVIDER` while
/// doing so, which would send it looking for `op` on a runner that only has
/// it inside the shell. The credential is inert to that resolution — the
/// shell's profile reads the environment — so it can stay on the step.
pub fn credentialed(step: Step<Run>) -> Step<Run> {
    step.add_env(TOKEN)
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

// ── Release-run expressions ─────────────────────────────────────────────────

/// The run's release tag, unwrapped for `if:` — already an expression
/// context, so `${{ }}` there is an error.
///
/// A planner output means nightly, which mints its tag in the `prepare`
/// job; without one the tag comes from the pushed ref or the dispatch
/// input.
fn tag_expr(tag: Option<&JobOutput>) -> String {
    match tag {
        Some(tag) => tag.expr(),
        None => "github.event_name == 'push' && github.ref_name || inputs.tag".to_owned(),
    }
}

/// The same tag, wrapped for use as a `with:` value.
pub fn tag(tag: Option<&JobOutput>) -> String {
    format!("${{{{ {tag} }}}}", tag = tag_expr(tag))
}

/// True when this run should publish. A build-only run has an empty tag.
pub fn tag_condition(tag: Option<&JobOutput>) -> Expression {
    Expression::new(match tag {
        Some(tag) => format!("{tag} != ''", tag = tag.expr()),
        None => format!("({tag}) != ''", tag = tag_expr(None)),
    })
}

/// The full set of choices offered by the `platforms` dispatch input.
pub fn dispatch_platforms() -> Vec<&'static str> {
    vec![
        "All", "Desktop", "macOS", "Linux", "Windows", "Mobile", "Android", "iOS",
    ]
}

/// Guards a job on the `platforms` dispatch input.
///
/// A job answers to `All`, its group (`Desktop`/`Mobile`), and its own
/// label. Given a planner verdict it additionally waits on that, which is
/// how a nightly with nothing new skips its builds.
pub fn guard(platform: Platform, should_build: Option<&JobOutput>) -> Expression {
    let group = match &platform {
        Platform::Android | Platform::iOS => "Mobile",
        Platform::Linux | Platform::macOS | Platform::Windows => "Desktop",
    };
    let labels = ["All", group, &platform.dispatch_label()];
    let choices = labels
        .iter()
        .map(|label| format!("\"{label}\""))
        .collect::<Vec<_>>()
        .join(",");
    let selected = format!("contains(fromJson('[{choices}]'), inputs.platforms)");

    Expression::new(match should_build {
        Some(should_build) => format!(
            "{planner} == 'true' && \
             (github.event_name != 'workflow_dispatch' || {selected})",
            planner = should_build.expr(),
        ),
        None => format!("github.event_name == 'push' || {selected}"),
    })
}

const BOTSWAIN_GIT_USERNAME: &str = "wp-botswain[bot]";
const BOTSWAIN_GIT_EMAIL: &str = "3633092+wp-botswain[bot]@users.noreply.github.com";

pub(crate) trait BotswainGitIdentity {
    fn with_botswain_git_identity(self) -> Self;
}

impl BotswainGitIdentity for Step<Run> {
    fn with_botswain_git_identity(self) -> Self {
        self.add_env(("GIT_AUTHOR_NAME", BOTSWAIN_GIT_USERNAME))
            .add_env(("GIT_COMMITTER_NAME", BOTSWAIN_GIT_USERNAME))
            .add_env(("GIT_AUTHOR_EMAIL", BOTSWAIN_GIT_EMAIL))
            .add_env(("GIT_COMMITTER_EMAIL", BOTSWAIN_GIT_EMAIL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    /// The scope names are what `secretspec.toml` declares under `[scopes]`;
    /// a rename on either side has to show up here.
    #[test]
    fn every_resolution_names_a_declared_scope() {
        let manifest = std::fs::read_to_string(
            std::path::Path::new(std::env!("CARGO_MANIFEST_DIR")).join("../../secretspec.toml"),
        )
        .expect("secretspec.toml");
        let resolutions = Platform::iter().map(Resolution::Release).chain([
            Resolution::Ci(CiScope::Cache),
            Resolution::Ci(CiScope::Botswain),
        ]);
        for resolution in resolutions {
            let header = format!("[scopes.{}]", resolution.scope());
            assert!(manifest.contains(&header), "no {header}");
            let profile = format!("[profiles.{}]", resolution.profile().name());
            assert!(manifest.contains(&profile), "no {profile}");
        }
    }
}
