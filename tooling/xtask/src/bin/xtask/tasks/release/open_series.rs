// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Opens a cut's series at its main pull request's merge commit and leaves
//! the app version written on it for the pull-request step. See
//! [`xtask::planners::release::open_series`].

use anyhow::Result;
use clap::Parser;
use xtask::{planners::release as planner, releases::version::Version, tools::github};

#[derive(Parser)]
pub struct Options {
    /// The app version the series opens at.
    #[arg(long)]
    version: String,

    /// The merge commit the series branches from.
    #[arg(long)]
    commit: String,
}

pub fn run(args: Options) -> Result<()> {
    let version = Version::parse(&args.version)?;
    let branch = version.release_branch();
    let landing = planner::open_series(&version, &args.commit)?;
    println!(
        "Opened {branch} at {commit}, with {version} written on it",
        commit = args.commit
    );
    println!("  {}", landing.describe(&branch));
    github::set_outputs_if_available(&landing)
}
