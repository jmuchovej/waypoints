//! Shared expressions and concurrency groups.

use gh_workflow::Concurrency;

/// One run per ref, superseding any run already in flight for that ref.
///
/// Correct for checks — a newer push makes the older answer irrelevant. Do not
/// reach for this on release workflows, where a cancelled run leaves published
/// artifacts half-written.
pub(crate) fn one_run_per_ref(prefix: &str) -> Concurrency {
    Concurrency::default()
        .group(format!("{prefix}-${{{{ github.ref }}}}"))
        .cancel_in_progress(true)
}
