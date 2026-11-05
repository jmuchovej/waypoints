// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Maintains the group file's generated schema. See
//! [`xtask::release_groups`].

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use std::fs;
use xtask::{
    release_groups::{self, SCHEMA_FILE},
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
    }
}
