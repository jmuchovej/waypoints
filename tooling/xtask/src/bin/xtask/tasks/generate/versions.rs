// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Writes every group's declared version into its member manifests and their
//! lockfile entries, from the group file as it stands — after editing the
//! file by hand, or adding a crate to a group. Changes nothing on a tree that
//! already agrees. See [`xtask::release_groups`].

use anyhow::Result;
use clap::Parser;
use xtask::{constants::GROUP_FILE, planners::release, release_groups, workspace};

#[derive(Parser)]
pub struct Options {}

pub fn run(args: Options) -> Result<()> {
    let Options {} = args;
    let metadata = workspace::load()?;
    // The file as it stands: the projection is what makes the manifests
    // agree with it.
    let groups = release_groups::load_unprojected(&metadata)?;
    let written = release::project_file(&metadata, &groups)?;
    if written.is_empty() {
        println!("every member manifest already declares its group's version from {GROUP_FILE}");
    }
    for path in written {
        println!("wrote {path}");
    }
    Ok(())
}
