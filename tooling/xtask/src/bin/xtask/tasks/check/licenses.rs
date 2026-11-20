// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Holds the tree to the licensing policy. The checks are [`xtask::license`]'s;
//! this runs them and prints what they found.

use anyhow::{Result, bail};
use clap::Parser;
use xtask::{license, workspace};

#[derive(Parser)]
pub struct Options {}

/// Checks manifests, headers, layering, license texts, and the third-party
/// registry.
pub fn run(args: Options) -> Result<()> {
    let root = workspace::load()?.workspace_root.into_std_path_buf();
    let report = license::check_headers(&root)?;

    for finding in &report.findings {
        eprintln!("{finding}");
    }
    if !report.findings.is_empty() {
        bail!("{} licensing finding(s)", report.findings.len());
    }
    println!(
        "{} crates and the tree agree with {} (SPDX list {})",
        report.crates,
        license::POLICY,
        spdx::identifiers::VERSION,
    );
    Ok(())
}
