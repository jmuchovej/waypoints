// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tags a merge commit: the app's tag, or every library tag a cut minted,
//! comma-separated as the planner publishes them. Run under an App token
//! minted after the merge, which is what raises each tag's `push` event.

use anyhow::Result;
use clap::Parser;
use xtask::tools::github;

#[derive(Parser)]
pub struct Options {
    /// The tags to create, comma-separated.
    #[arg(long)]
    tags: String,

    /// The commit every tag points at.
    #[arg(long)]
    commit: String,
}

/// The tags in `list`, trimmed, with blanks dropped.
fn tags(list: &str) -> Vec<&str> {
    list.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect()
}

pub fn run(args: Options) -> Result<()> {
    for tag in tags(&args.tags) {
        github::create_tag(tag, &args.commit)?;
        println!("Tagged {tag} at {}", args.commit);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The planner publishes the group tags comma-separated; a stray space or
    /// trailing comma must not become a tag.
    #[test]
    fn the_list_is_read_as_the_planner_publishes_it() {
        assert_eq!(
            tags("bedrock-v0.1.1, domain-v0.1.1,,"),
            ["bedrock-v0.1.1", "domain-v0.1.1"]
        );
        assert_eq!(tags("v0.1.0-pre.1"), ["v0.1.0-pre.1"]);
        assert!(tags("").is_empty());
    }
}
