//! Shared expressions, secret handles, and concurrency groups.
//! Trigger plumbing shared by workflow modules.

use gh_workflow::{Concurrency, Permissions, Workflow};

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

secret!(SOPS_AGE_KEY);
secret!(GITHUB_TOKEN);
env!(ANDROID_HOME);
env!(WP_BOTSWAIN_APP_ID);
env!(WP_BOTSWAIN_PRIVATE_KEY);

/// The secretspec profile holding those credentials.
pub const AUTOMATION_PROFILE: &str = "automation";

/// Pins JavaScript actions to Node 24; the runner otherwise falls back to its
/// own default, which they fail to start under.
pub const FORCE_NODE24: (&str, &str) = ("FORCE_JAVASCRIPT_ACTIONS_TO_NODE24", "true");

/// Selects the sops backend for every `secretspec` call in the workflow.
/// secretspec treats a manifest's `providers` list as an allowlist, not a
/// selector — without this it resolves against `env://` and every sops-backed
/// secret comes back missing.
pub const SECRETSPEC_PROVIDER: (&str, &str) = ("SECRETSPEC_PROVIDER", "sops");

pub const SOPS_AGE_KEY_ENV: (&str, &str) = ("SOPS_AGE_KEY", SOPS_AGE_KEY);

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
        .add_env(SECRETSPEC_PROVIDER)
}
