// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Checks what a pull request claims about versions. See
//! [`xtask::version_integrity`].

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use std::{fs, path::PathBuf};
use xtask::{planners::test as test_plan, version_integrity, workspace};

#[derive(Parser)]
pub struct Options {
    /// The `pull_request` event payload. Actions sets the variable on every
    /// run; pass the flag to try a title and description locally.
    #[arg(long, env = "GITHUB_EVENT_PATH")]
    event: PathBuf,

    /// Revision to diff against. Defaults to the pull request's merge base.
    #[arg(long)]
    base: Option<String>,
}

pub fn run(args: Options) -> Result<()> {
    let payload = fs::read_to_string(&args.event)
        .with_context(|| format!("Failed to read {}", args.event.display()))?;
    let base = match args.base {
        Some(base) => base,
        None => test_plan::default_base()?,
    };

    let violations = version_integrity::run(&workspace::load()?, &payload, &base)?;
    if violations.is_empty() {
        println!("Nothing this pull request claims about versions needs correcting.");
        return Ok(());
    }

    bail!(
        "{count} version-integrity violation(s):\n  {list}",
        count = violations.len(),
        list = violations.join("\n  "),
    )
}
