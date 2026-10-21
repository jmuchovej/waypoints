//! Best-effort cleanup of the artifacts a shortfall run uploaded. Atomicity
//! never depends on this succeeding — no manifest ever named these keys,
//! so they are unreferenced and unadvertised either way — which is what
//! makes a failed delete safe to ignore rather than something that must
//! fail the job. See the design's "A withdrawal deletes the keys it wrote,
//! best-effort" decision.
//!
//! Attempts every key the channel's required targets could have produced
//! (see [`xtask::releases::artifacts::published_keys`]), not only the ones
//! this particular run actually wrote: deleting a key that was never
//! uploaded is an ordinary S3 no-op, not an error, so there is nothing to
//! gain from tracking which targets actually succeeded before asking the
//! store to remove them.

use anyhow::{Context as _, Result};
use clap::Parser;
use xtask::{
    releases::version::Version,
    releases::{artifacts, store},
};

#[derive(Parser)]
pub struct Options {
    /// The release tag this run failed or fell short of. The channel and
    /// series are derived from it, never supplied — the same rule the
    /// uploader and manifest writer follow.
    #[arg(long)]
    tag: String,

    /// How long the derived store credential stays valid, in seconds.
    #[arg(long, default_value_t = store::DEFAULT_TTL_SECONDS)]
    ttl_seconds: u64,
}

pub fn run(args: Options) -> Result<()> {
    let channel = args.tag.parse::<Version>()?.channel();
    let keys: Vec<String> = artifacts::published_keys(channel, &args.tag)?
        .into_values()
        .collect();

    let client = store::create_r2_client(channel, args.ttl_seconds)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    let outcome = runtime.block_on(client.delete_iter(keys.clone()));
    report(outcome, &args.tag, keys.len())
}

/// Turns the store's delete outcome into this command's own exit status —
/// always `Ok`, regardless of `outcome`. A failed delete leaves an orphan
/// that tiers down to the cheapest storage class and is never served
/// either way, since no manifest names it; see the module doc. That makes
/// it unlike every other failure this command could raise, which is why it
/// is handled here rather than with `?` at the call site.
fn report(outcome: opendal::Result<()>, tag: &str, attempted: usize) -> Result<()> {
    match outcome {
        Ok(()) => println!("withdrew {attempted} candidate key(s) for {tag}"),
        Err(error) => eprintln!("::warning::cleanup of {tag} left orphans: {error}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shortfall run's cleanup must not itself become a second failure —
    /// see the module doc. `report` is where that guarantee lives, since
    /// `run` can no longer propagate a delete error once `?` is out of the
    /// path.
    #[test]
    fn a_failed_object_delete_does_not_fail_the_command() {
        let error = opendal::Error::new(opendal::ErrorKind::Unexpected, "simulated failure");
        assert!(report(Err(error), "v0.1.1", 3).is_ok());
    }
}
