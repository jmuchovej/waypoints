//! Uploads the staged artifacts to the `releases` bucket through the shared
//! client in [`xtask::releases::store`]. The channel is derived from the
//! tag's version rather than accepted as an argument, and every staged file
//! is checked against the keys the scheme produces (see
//! [`xtask::releases::artifacts`]) before any credential is derived.

use anyhow::{Context as _, Result};
use clap::Parser;
use std::path::PathBuf;
use xtask::{
    releases::version::Version,
    releases::{artifacts, store},
};

#[derive(Parser)]
pub struct Options {
    /// The flat directory every build's `release-stage` output merges into;
    /// each file is uploaded to the key its name and extension determine.
    #[arg(long)]
    from: PathBuf,

    /// The release tag, e.g. `v0.1.1` or `v0.2.0-nightly.3+2026.10.19`. Both
    /// the store's version segment and the saved filename come from this
    /// exactly; the channel and series are derived from it, never supplied.
    #[arg(long)]
    tag: String,

    /// How long the derived store credential stays valid, in seconds.
    #[arg(long, default_value_t = store::DEFAULT_TTL_SECONDS)]
    ttl_seconds: u64,
}

pub fn run(args: Options) -> Result<()> {
    let channel = args.tag.parse::<Version>()?.channel();
    let files = artifacts::staged_files(&args.from, channel, &args.tag)?;

    let client = store::create_r2_client(channel, args.ttl_seconds)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    for (path, key) in files {
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        runtime.block_on(store::upload(&client, &key, bytes))?;
        println!("uploaded {} to {key}", path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The CLI default is the same lifetime `store` settles on; raising it
    /// is a flag, not a code change.
    #[test]
    fn the_cli_defaults_to_the_store_s_ttl() {
        let args = Options::parse_from(["xtask", "--from", "release", "--tag", "v0.1.1"]);
        assert_eq!(args.ttl_seconds, store::DEFAULT_TTL_SECONDS);
    }

    /// A build's output is never uploaded directly; only a staged tree is.
    #[test]
    fn the_cli_uploads_only_a_staged_tree() {
        let tag = ["--tag", "v0.1.1"];
        assert!(Options::try_parse_from(["xtask"].iter().chain(&tag)).is_err());
        assert!(
            Options::try_parse_from(["xtask", "--platform", "ios"].iter().chain(&tag)).is_err()
        );
    }
}
