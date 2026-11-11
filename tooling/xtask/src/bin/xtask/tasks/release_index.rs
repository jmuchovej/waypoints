// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Writes a published tag's release-index entry to the working tree, for the
//! pull-request step to land on `main`. See [`xtask::releases::index`].

use anyhow::{Context as _, Result};
use clap::Parser;
use std::fs;
use xtask::{
    releases::{index, version::Version},
    tools::github,
    workspace,
};

#[derive(Parser)]
pub struct Options {
    /// The tag to record, exactly as created: `v{version}`.
    #[arg(long)]
    tag: String,
    /// Compute and print the entry without writing it.
    #[arg(long)]
    dry_run: bool,
}

pub fn run(args: Options) -> Result<()> {
    let version: Version = args.tag.parse()?;
    let (rev, manifest) = index::tagged_manifest(&args.tag)?;
    let channel = index::classify(&args.tag, &version, &manifest)?;
    println!("{tag} is a {channel} release at {rev}", tag = args.tag);

    let hash = index::source_hash(&rev)?;
    println!("  source: {hash}");

    let entry = index::Entry {
        tag: args.tag.clone(),
        rev,
        hash,
    };
    let path = index::Entry::path(channel);
    let content = entry.render()?;
    println!("{path}:\n{content}");

    let landing = entry.landing(channel);
    println!("  {}", landing.describe("main"));

    if args.dry_run {
        println!("  (dry run — nothing written)");
        return Ok(());
    }

    let file = workspace::load()?.workspace_root.join(&path);
    fs::write(&file, content).with_context(|| format!("Failed to write {file}"))?;
    println!("  wrote {path}");
    github::set_outputs_if_available(&landing)
}
