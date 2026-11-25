// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Regenerates the group file's JSON Schema, which tombi lints the file
//! against and `version-integrity` holds byte-for-byte to the types it is
//! derived from. See [`xtask::release_groups::schema`].

use anyhow::{Context as _, Result};
use clap::Parser;
use std::fs;
use xtask::{constants::SCHEMA_FILE, release_groups, workspace};

#[derive(Parser)]
pub struct Options {}

pub fn run(args: Options) -> Result<()> {
    let Options {} = args;
    let path = workspace::load()?.workspace_root.join(SCHEMA_FILE);
    let parent = path.parent().context("the schema lives in a folder")?;
    fs::create_dir_all(parent).with_context(|| format!("Failed to create {parent}"))?;
    fs::write(&path, release_groups::schema()?)
        .with_context(|| format!("Failed to write {path}"))?;
    println!("wrote {path}");
    Ok(())
}
