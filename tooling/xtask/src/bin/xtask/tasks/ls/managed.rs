// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `ls managed` — every generated path the tree carries: the ledger's outputs,
//! then the files devenv materializes. See [`xtask::constants::GENERATED`]
//! and [`xtask::tools::devenv`].

use anyhow::Result;
use clap::Parser;
use xtask::{constants::GENERATED, tools::devenv};

#[derive(Parser)]
pub struct Options {
    /// Restrict devenv's files to the ones it copies into the tree, dropping
    /// store symlinks. Wanted by callers that diff contents; a symlink into
    /// the store cannot drift.
    #[arg(long)]
    copied: bool,
}

/// Prints one path per line, globs as written, so a caller can consume it
/// with a plain `for`.
pub fn run(args: Options) -> Result<()> {
    for output in GENERATED.iter().flat_map(|entry| entry.outputs) {
        println!("{output}");
    }

    let files = if args.copied {
        devenv::copied_files()?
    } else {
        devenv::managed_files()?
    };
    for file in files {
        println!("{file}");
    }

    Ok(())
}
