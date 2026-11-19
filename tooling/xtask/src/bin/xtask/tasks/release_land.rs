// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Takes a bot's pull request from opened to landed, one step at a time, so a
//! job can mint a fresh App token between the wait and the tag: an installation
//! token lasts an hour, and the required checks can take most of that. A cut
//! lands twice — its main-side summary, then its series — and between the two
//! tags each library it minted and opens the series at that merge commit.

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
    /// Tags a merge commit: the app's tag, or every library tag a cut
    /// minted, comma-separated as the planner publishes them.
    Tag {
        #[arg(long)]
        tags: String,
        #[arg(long)]
        commit: String,
    },
    /// Opens a cut's series at its main pull request's merge commit and leaves
    /// the app version written on it for the pull-request step.
    OpenSeries {
        #[arg(long)]
        version: String,
        #[arg(long)]
        commit: String,
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
        Action::Tag { tags, commit } => {
            for tag in tags.split(',').map(str::trim).filter(|tag| !tag.is_empty()) {
                github::create_tag(tag, &commit)?;
                println!("Tagged {tag} at {commit}");
            }
            Ok(())
        }
        Action::OpenSeries { version, commit } => {
            let version = Version::parse(&version)?;
            let branch = version.release_branch();
            let landing = planner::open_series(&version, &commit)?;
            println!("Opened {branch} at {commit}, with {version} written on it");
            println!("  {}", landing.describe(&branch));
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
            &["release-land", "tag", "--tags", "v0.2.0", "--commit", "abc"],
            &[
                "release-land",
                "tag",
                "--tags",
                "bedrock-v0.2.0,domain-v0.1.1",
                "--commit",
                "abc",
            ],
            &[
                "release-land",
                "open-series",
                "--version",
                "0.2.0-pre.1",
                "--commit",
                "abc",
            ],
        ] {
            assert!(Options::try_parse_from(args).is_ok(), "{args:?}");
        }

        for args in [
            &["release-land"][..],
            &["release-land", "wait"],
            &["release-land", "bump-main", "--version", "0.3.0-nightly"],
            &[
                "release-land",
                "tag-groups",
                "--tags",
                "v0.2.0",
                "--commit",
                "abc",
            ],
            &["release-land", "tag", "--tag", "v0.2.0", "--commit", "abc"],
            &["release-land", "auto-merge", "--pull-request", "seven"],
        ] {
            assert!(Options::try_parse_from(args).is_err(), "{args:?}");
        }
    }
}
