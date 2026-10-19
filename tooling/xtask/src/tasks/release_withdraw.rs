//! Best-effort cleanup of the artifacts a shortfall run uploaded. Atomicity
//! never depends on this succeeding — no manifest ever named these keys,
//! so they are unreferenced and unadvertised either way — which is what
//! makes a failed delete safe to ignore rather than something that must
//! fail the job. See the design's "A withdrawal deletes the keys it wrote,
//! best-effort" decision.
//!
//! Attempts every key the channel's required targets could have produced,
//! not only the ones this particular run actually wrote: deleting a key
//! that was never uploaded is an ordinary S3 no-op, not an error, so there
//! is nothing to gain from tracking which targets actually succeeded
//! before asking the store to remove them.

use crate::tasks::{release_plan, release_store, workflows::Channel};
use anyhow::{Context as _, Result};
use cargo_metadata::semver::Version;
use clap::Parser;

#[derive(Parser)]
pub struct ReleaseWithdrawArgs {
    /// The release tag this run failed or fell short of. The channel and
    /// series are derived from it, never supplied — the same rule the
    /// uploader and manifest writer follow.
    #[arg(long)]
    tag: String,

    /// How long the derived store credential stays valid, in seconds.
    #[arg(long, default_value_t = release_store::DEFAULT_TTL_SECONDS)]
    ttl_seconds: u64,
}

pub fn run(args: ReleaseWithdrawArgs) -> Result<()> {
    let version = tag_version(&args.tag)?;
    let channel = release_plan::channel_of(&version);
    let keys = candidate_keys(channel, &args.tag);

    let client = release_store::create_r2_client(channel, args.ttl_seconds)?;

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

fn tag_version(tag: &str) -> Result<Version> {
    let version = tag
        .strip_prefix('v')
        .with_context(|| format!("tag `{tag}` has no leading `v`"))?;
    Version::parse(version).with_context(|| format!("tag `{tag}` is not a valid version"))
}

/// Every key this channel's required targets could have produced for
/// `tag`, in the same `{channel}/{series}/{os}/{arch}/waypoints-{tag}.{ext}`
/// shape the uploader writes.
fn candidate_keys(channel: Channel, tag: &str) -> Vec<String> {
    let version = tag_version(tag).expect("validated by `run` already");
    let series = release_plan::series_of(&version);

    let mut keys = Vec::new();
    for platform in channel.required_targets() {
        let target = platform.published_target();
        for extension in target.extensions {
            keys.push(format!(
                "{channel}/{series}/{os}/{arch}/waypoints-{tag}.{extension}",
                channel = channel.flavor(),
                os = target.os,
                arch = target.arch,
            ));
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_keys_cover_every_required_target_and_extension() {
        let keys = candidate_keys(Channel::Stable, "v0.1.1");
        // Every required target contributes at least one key, and Android
        // contributes two (apk and aab) under the same target.
        assert!(keys.len() > Channel::Stable.required_targets().len());
        assert!(keys.contains(&"stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg".to_owned()));
        assert!(keys.contains(&"stable/v0.1.x/android/arm64/waypoints-v0.1.1.apk".to_owned()));
        assert!(keys.contains(&"stable/v0.1.x/android/arm64/waypoints-v0.1.1.aab".to_owned()));
    }

    #[test]
    fn candidate_keys_stay_under_the_channel_s_own_prefix() {
        for key in candidate_keys(Channel::Nightly, "v0.2.0-nightly.3+2026.10.19") {
            assert!(key.starts_with("nightly/"), "{key}");
        }
    }

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
