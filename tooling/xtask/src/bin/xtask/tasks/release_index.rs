// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Writes a published tag's release-index entry to the working tree, for the
//! pull-request step to land on `main` — or, on a dry run, renders the entry
//! a tag would receive, to a directory or to the log. See
//! [`xtask::releases::index`].

use anyhow::{Context as _, Result};
use clap::Parser;
use std::{fs, path::PathBuf};
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
    /// The commit to render the entry from, for a tag that does not exist
    /// yet. Needs `--dry-run`: a real write records only a tag that exists.
    #[arg(long, requires = "dry_run")]
    rev: Option<String>,
    /// Compute and print the entry without writing it to the tree.
    #[arg(long)]
    dry_run: bool,
    /// The directory a dry run writes the entry under, at its path in the
    /// tree: `nix/releases/{channel}.json`.
    #[arg(long, requires = "dry_run")]
    into: Option<PathBuf>,
}

pub fn run(args: Options) -> Result<()> {
    let version: Version = args.tag.parse()?;
    let reference = args.rev.as_deref().unwrap_or(&args.tag);
    let (rev, manifest) = index::manifest_at(reference)?;
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
        match &args.into {
            Some(dir) => {
                let file = dir.join(&path);
                let parent = file.parent().context("the entry's path has a parent")?;
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
                fs::write(&file, content).with_context(|| format!("writing {}", file.display()))?;
                println!("  (dry run — rendered to {})", file.display());
            }
            None => println!("  (dry run — nothing written)"),
        }
        return Ok(());
    }

    let file = workspace::load()?.workspace_root.join(&path);
    fs::write(&file, content).with_context(|| format!("Failed to write {file}"))?;
    println!("  wrote {path}");
    github::set_outputs_if_available(&landing)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rev stands in for a tag only on a dry run: a real write records a
    /// tag that exists, pointing where it points.
    #[test]
    fn a_rev_is_only_for_a_dry_run() {
        let dry = Options::try_parse_from([
            "release-index",
            "--tag",
            "v0.2.0-nightly.20261109",
            "--rev",
            "abc123",
            "--dry-run",
            "--into",
            "out",
        ])
        .unwrap();
        assert_eq!(dry.rev.as_deref(), Some("abc123"));
        assert!(dry.dry_run);
        assert!(
            Options::try_parse_from(["release-index", "--tag", "v0.2.0", "--rev", "abc123"])
                .is_err()
        );
        assert!(
            Options::try_parse_from(["release-index", "--tag", "v0.2.0", "--into", "out"]).is_err()
        );
    }
}
