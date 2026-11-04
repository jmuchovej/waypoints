// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Holds the tree to the licensing policy, and publishes the contributor
//! agreement. The checks are [`xtask::license`]'s; this picks what to run and
//! prints what it found.

use anyhow::{Context as _, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use xtask::{
    license::{self, Agreement, Branch, GitHubGist, Published},
    workspace,
};

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Checks manifests, headers, layering, license texts, the third-party
    /// registry, and the published agreement. The default.
    Check(CheckOptions),
    /// Regenerates `THIRD-PARTY-NOTICES.md` from the registry and the lockfiles.
    Notices,
    /// Writes the repository's agreement to the Gist CLA assistant displays.
    /// Approve the code it prints in a browser; nothing is stored.
    Publish,
}

#[derive(Default, Parser)]
struct CheckOptions {
    /// Judge an unpublished agreement version as this branch would. Defaults
    /// to what `$GITHUB_REF` says, and to a pull request off a runner.
    #[arg(long, value_enum)]
    branch: Option<BranchArg>,
}

#[derive(Clone, Copy, ValueEnum)]
enum BranchArg {
    PullRequest,
    Main,
}

pub fn run(args: Options) -> Result<()> {
    let root = workspace::load()?.workspace_root.into_std_path_buf();
    match args.action {
        None => check(&root, CheckOptions::default()),
        Some(Action::Check(options)) => check(&root, options),
        Some(Action::Notices) => notices(&root),
        Some(Action::Publish) => publish(&root),
    }
}

fn check(root: &std::path::Path, options: CheckOptions) -> Result<()> {
    let branch = match options.branch {
        Some(BranchArg::PullRequest) => Branch::PullRequest,
        Some(BranchArg::Main) => Branch::Main,
        None => Branch::from_environment(),
    };
    let report = license::check(root, branch, &GitHubGist)?;

    for note in &report.notes {
        println!("note: {note}");
    }
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
    let contents = license::notices::generate(root)?;
    let path = root.join(license::notices::NOTICES);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, contents).with_context(|| format!("Failed to write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn publish(root: &std::path::Path) -> Result<()> {
    let repository = Agreement::from_repository(root)?;
    match license::publish(&repository, &GitHubGist)? {
        Published::AlreadyCurrent => println!("the published agreement is already current"),
        Published::Revision(revision) => println!("published as revision {revision}"),
    }
    Ok(())
}
