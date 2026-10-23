//! Writes a channel's manifest once every required target has uploaded. See
//! [`xtask::releases::manifest`].

use anyhow::{Context as _, Result};
use clap::Parser;
use xtask::{
    releases::version::Version,
    releases::{manifest::Manifest, store},
};

#[derive(Parser)]
pub struct Options {
    /// The release tag this manifest publishes. The channel and series are
    /// derived from it, never supplied — the same rule the uploader follows.
    #[arg(long)]
    tag: String,

    /// How long the derived store credential stays valid, in seconds.
    #[arg(long, default_value_t = store::DEFAULT_TTL_SECONDS)]
    ttl_seconds: u64,
}

pub fn run(args: Options) -> Result<()> {
    let version: Version = args.tag.parse()?;
    let channel = version.channel();
    let manifest = Manifest::build(channel, &version);
    let body = manifest.render()?;

    let client = store::create_r2_client(channel, args.ttl_seconds)?;
    let key = store::manifest_key(channel);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;
    runtime.block_on(async {
        client
            .write(&key, body)
            .await
            .with_context(|| format!("writing the manifest to {key}"))
    })?;

    println!("wrote {key} naming {}", manifest.version);
    Ok(())
}
