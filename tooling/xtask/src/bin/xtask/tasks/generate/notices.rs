// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Regenerates `THIRD-PARTY-NOTICES.md` from the registry and the lockfiles.
//! The text is [`xtask::license`]'s; this writes it where it is committed.

use anyhow::{Context as _, Result};
use clap::Parser;
use std::fs;
use xtask::{constants::NOTICES, license, workspace};

#[derive(Parser)]
pub struct Options {}

pub fn run(args: Options) -> Result<()> {
    let root = workspace::load()?.workspace_root.into_std_path_buf();
    let contents = license::generate_notices(&root)?;
    let path = root.join(NOTICES);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, contents).with_context(|| format!("Failed to write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}
