// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lands a bot's pull request: has GitHub merge it once its required checks
//! pass and, unless told not to, waits for the merge and publishes the merge
//! commit for the step that tags it. A release waits, so its tag names a
//! commit the planners' first-parent walk reaches; the index's entry is
//! landed and left.

use anyhow::Result;
use clap::Parser;
use serde::Serialize;
use xtask::tools::github;

#[derive(Parser)]
pub struct Options {
    /// The pull request to land.
    #[arg(long)]
    pull_request: u64,

    /// Enable the merge and return without waiting for it.
    #[arg(long)]
    no_wait: bool,
}

/// What a waited-for landing publishes.
#[derive(Serialize)]
struct Merged {
    merge_commit: String,
}

pub fn run(args: Options) -> Result<()> {
    let number = args.pull_request;
    github::enable_auto_merge(number)?;
    println!("#{number} merges when its required checks pass");
    if args.no_wait {
        return Ok(());
    }

    println!("Waiting for #{number} to merge");
    let merge_commit = github::wait_for_merge(number)?;
    println!("#{number} merged as {merge_commit}");
    github::set_outputs_if_available(&Merged { merge_commit })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator reads the merge commit back as `merge_commit`; the name
    /// is the contract between this command and the tag step after it.
    #[test]
    fn the_landing_publishes_the_merge_commit_under_its_name() {
        let published = serde_json::to_value(Merged {
            merge_commit: "abc".to_owned(),
        })
        .unwrap();
        assert_eq!(published, serde_json::json!({ "merge_commit": "abc" }));
    }
}
