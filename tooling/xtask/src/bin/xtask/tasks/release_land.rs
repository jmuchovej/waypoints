// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Takes a bot's pull request from opened to landed, one step at a time, so a
//! job can mint a fresh App token between the wait and the tag: an installation
//! token lasts an hour, and the required checks can take most of that.

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;
use xtask::{planners::release as planner, releases::version::Version, tools::github};

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Has GitHub merge the pull request once its required checks pass.
    AutoMerge {
        #[arg(long)]
        pull_request: u64,
    },
    /// Waits for the pull request to merge and publishes its merge commit.
    Wait {
        #[arg(long)]
        pull_request: u64,
    },
    /// Tags a merge commit.
    Tag {
        #[arg(long)]
        tag: String,
        #[arg(long)]
        commit: String,
    },
    /// Leaves `main`'s post-cut bump in the tree for the pull-request step.
    BumpMain {
        #[arg(long)]
        version: String,
    },
}

#[derive(Serialize)]
struct Merged {
    merge_commit: String,
}

pub fn run(args: Options) -> Result<()> {
    match args.action {
        Action::AutoMerge { pull_request } => {
            github::enable_auto_merge(pull_request)?;
            println!("#{pull_request} merges when its required checks pass");
            Ok(())
        }
        Action::Wait { pull_request } => {
            println!("Waiting for #{pull_request} to merge");
            let merge_commit = github::wait_for_merge(pull_request)?;
            println!("#{pull_request} merged as {merge_commit}");
            github::set_outputs_if_available(&Merged { merge_commit })
        }
        Action::Tag { tag, commit } => {
            github::create_tag(&tag, &commit)?;
            println!("Tagged {tag} at {commit}");
            Ok(())
        }
        Action::BumpMain { version } => {
            let landing = planner::bump_main(&Version::parse(&version)?)?;
            println!("Wrote {version} on main");
            println!("  {}", landing.describe("main"));
            github::set_outputs_if_available(&landing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cli_names_one_step_of_a_landing() {
        for args in [
            &["release-land", "auto-merge", "--pull-request", "7"][..],
            &["release-land", "wait", "--pull-request", "7"],
            &["release-land", "tag", "--tag", "v0.2.0", "--commit", "abc"],
            &["release-land", "bump-main", "--version", "0.3.0-nightly"],
        ] {
            assert!(Options::try_parse_from(args).is_ok(), "{args:?}");
        }

        for args in [
            &["release-land"][..],
            &["release-land", "wait"],
            &["release-land", "auto-merge", "--pull-request", "seven"],
        ] {
            assert!(Options::try_parse_from(args).is_err(), "{args:?}");
        }
    }
}
