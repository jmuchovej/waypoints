// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `release undo --tag <v> --commit <sha>`: reverts the bump a withdrawn
//! release was minted from, leaving the revert in the working tree for the
//! pull-request action and publishing its landing. For a cut's `-pre.1` it
//! deletes the library tags minted at the cut's `main` write and the series
//! itself, and reverts that write on `main`, where its landing goes. It
//! deletes no `v*` tag: the workflow step does. See [`xtask::releases::undo`].

use anyhow::{Context as _, Result};
use clap::Parser;
use std::fs;
use xtask::{
    constants::{GROUP_FILE, MAIN},
    releases::undo::{self, Kind},
    templates,
    tools::{git, github},
    workspace,
};

#[derive(Parser)]
pub struct Options {
    /// The withdrawn release's tag.
    #[arg(long)]
    tag: String,

    /// The merge commit of the write that declared the tag's version.
    #[arg(long)]
    commit: String,
}

/// Where the pull request's body was written: a summary's headers span
/// lines, which a step output cannot carry.
#[derive(serde::Serialize)]
struct BodyPath {
    body_path: String,
}

pub fn run(args: Options) -> Result<()> {
    let root = workspace::load()?.workspace_root.into_std_path_buf();
    let tags = git::remote_tags_at("*-v*")?;
    let undo = undo::plan(&root, &args.tag, &args.commit, &tags)?;

    match &undo.kind {
        Kind::Series { branch } => {
            git::check_out_latest(branch)?;
            let paths = undo::revert(&root, &undo.commit)?;
            let landing = undo.landing(branch, &paths, &[]);
            println!("reverted {} on {branch}", undo.version);
            println!("  {}", landing.describe(branch));
            github::set_outputs_if_available(&landing)?;
            publish_body(&templates::must("pull-request.undo", tera::Context::new()))
        }
        Kind::Cut {
            series,
            main_write,
            library_tags,
        } => {
            // The revert first: it touches only the working tree, so a
            // failure here leaves the remote as it was.
            git::check_out_latest(MAIN)?;
            let before = read(&root.join(GROUP_FILE))?;
            let paths = undo::revert(&root, main_write)?;
            let headers = undo::restored_headers(&before, &read(&root.join(GROUP_FILE))?)?;
            for tag in library_tags {
                github::delete_tag(tag)?;
                println!("deleted {tag}");
            }
            github::delete_branch(series)?;
            println!("deleted {series}");
            let landing = undo.landing(MAIN, &paths, &headers);
            println!("reverted the cut of {} on main", undo.version);
            println!("  {}", landing.describe(MAIN));
            for header in &headers {
                println!("    {header}");
            }
            github::set_outputs_if_available(&landing)?;
            let mut body = templates::must("pull-request.undo", tera::Context::new());
            if headers.len() > 1 {
                body = format!("{body}\n\n{}\n", headers.join("\n"));
            }
            publish_body(&body)
        }
    }
}

fn read(path: &std::path::Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))
}

fn publish_body(text: &str) -> Result<()> {
    let path = std::env::temp_dir().join("release-undo-body.md");
    fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    github::set_outputs_if_available(&BodyPath {
        body_path: path.to_string_lossy().into_owned(),
    })
}
