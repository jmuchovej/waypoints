//! Shared expressions, secret handles, and concurrency groups.

use gh_workflow::Concurrency;

macro_rules! secret {
    ($name:ident) => {
        pub const $name: &str = concat!("${{ secrets.", stringify!($name), " }}");
    };
}

secret!(SOPS_AGE_KEY);
secret!(GITHUB_TOKEN);

/// Node 24 is what our JavaScript actions target; without this, the runner
/// downgrades them to its default and they fail to start.
pub const FORCE_NODE24: (&str, &str) = ("FORCE_JAVASCRIPT_ACTIONS_TO_NODE24", "true");

/// One run per ref, superseding any run already in flight for that ref.
///
/// Correct for checks — a newer push makes the older answer irrelevant. Do not
/// reach for this on release workflows, where a cancelled run leaves published
/// artifacts half-written.
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
