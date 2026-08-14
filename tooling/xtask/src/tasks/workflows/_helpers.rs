//! Shared expressions, secret handles, and concurrency groups.
//! Trigger plumbing shared by workflow modules.

use gh_workflow::Concurrency;

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
