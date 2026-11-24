// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use xtask::dev::demo;

#[derive(Parser)]
pub struct Options {
    /// Where to write the app-data directory; must not exist or be empty.
    dir: PathBuf,

    /// A Bundle directory to ingest on top of `getting-started`, which is
    /// always present.
    #[arg(long)]
    bundle: Option<PathBuf>,
}

/// Prints the territory's id and mark count, tab-separated, for a caller to
/// read back.
pub fn run(args: Options) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let demo = runtime.block_on(demo::write(&args.dir, args.bundle.as_deref()))?;
    println!("{}\t{}", demo.id, demo.marks);
    Ok(())
}
