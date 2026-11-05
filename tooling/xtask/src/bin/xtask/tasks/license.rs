// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Holds the tree to the licensing policy. The checks are [`xtask::license`]'s;
//! this picks what to run and prints what it found.

use anyhow::{Context as _, Result, bail};
use clap::{Parser, Subcommand};
use std::fs;
use xtask::{license, workspace};

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Checks manifests, headers, layering, license texts, and the third-party
    /// registry. The default.
    Check,
    /// Regenerates `THIRD-PARTY-NOTICES.md` from the registry and the lockfiles.
    Notices,
}

pub fn run(args: Options) -> Result<()> {
    let root = workspace::load()?.workspace_root.into_std_path_buf();
    match args.action {
        None | Some(Action::Check) => check(&root),
        Some(Action::Notices) => notices(&root),
    }
}

fn check(root: &std::path::Path) -> Result<()> {
    let report = license::check_headers(root)?;

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

fn notices(root: &std::path::Path) -> Result<()> {
    let contents = license::generate_notices(root)?;
    let path = root.join(license::NOTICES_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, contents).with_context(|| format!("Failed to write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}
