// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Maintains the group file's generated schema, and projects the versions it
//! declares onto the member manifests. See [`xtask::release_groups`].

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use std::fs;
use xtask::{
    planners::release,
    release_groups::{self, GROUP_FILE, SCHEMA_FILE},
    workspace,
};

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Writes the JSON Schema tombi lints the group file against.
    Schema,
    /// Writes every group's declared version into its member manifests and
    /// their lockfile entries, from the file as it stands — after editing the
    /// file by hand, or adding a crate to a group. Changes nothing on a tree
    /// that already agrees.
    Project,
}

pub fn run(args: Options) -> Result<()> {
    match args.command {
        Command::Schema => {
            let path = workspace::load()?.workspace_root.join(SCHEMA_FILE);
            let parent = path.parent().context("the schema lives in a folder")?;
            fs::create_dir_all(parent).with_context(|| format!("Failed to create {parent}"))?;
            fs::write(&path, release_groups::schema()?)
                .with_context(|| format!("Failed to write {path}"))
        }
        Command::Project => {
            let metadata = workspace::load()?;
            let groups = release_groups::load(&metadata)?;
            let written = release::project_file(&metadata, &groups)?;
            if written.is_empty() {
                println!(
                    "every member manifest already declares its group's version from {GROUP_FILE}"
                );
            }
            for path in written {
                println!("wrote {path}");
            }
            Ok(())
        }
    }
}
