// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

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
    /// The flat directory every build's `release stage` output merges into;
    /// each file is uploaded to the key its name and extension determine.
    #[arg(long)]
    from: PathBuf,

    /// The release tag, e.g. `v0.1.1` or `v0.2.0-nightly.20261019`. Both
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
