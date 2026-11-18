// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Measures each library group a pull request touches against the group's
//! last tag, under the stable flavor, and fails on a break the group's
//! header does not declare. See [`xtask::semver_check`].

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use std::{fs, path::PathBuf};
use xtask::{
    planners::test as test_plan,
    release_groups,
    semver_check::{self, Verdict},
    tools::git,
    version_integrity::{self, PullRequest},
    workspace,
};

#[derive(Parser)]
pub struct Options {
    /// The `pull_request` event payload. Actions sets the variable on every
    /// run; pass the flag to try a title and description locally.
    #[arg(long, env = "GITHUB_EVENT_PATH")]
    event: PathBuf,

    /// Revision to diff against. Defaults to the pull request's merge base.
    #[arg(long)]
    base: Option<String>,
}

pub fn run(args: Options) -> Result<()> {
    let payload = fs::read_to_string(&args.event)
        .with_context(|| format!("Failed to read {}", args.event.display()))?;
    let pull_request = PullRequest::from_event(&payload)?;
    let base = match args.base {
        Some(base) => base,
        None => test_plan::default_base()?,
    };

    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let changes = version_integrity::changes(&base)?;
    let touched = groups.touched(changes.iter().map(|change| change.path.as_str()));
    if touched.is_empty() {
        println!("No release group's paths changed; nothing to measure.");
        return Ok(());
    }

    let mut failures = Vec::new();
    for group in &touched {
        let prefix = semver_check::prefix(&groups, group)?;
        let tags = git::remote_tags_like(&format!("{prefix}*"))?;
        let verdict = match semver_check::baseline(prefix, &tags) {
            None => Verdict::Skipped {
                group: group.clone(),
                prefix: prefix.to_owned(),
            },
            Some(tag) => {
                let broke = semver_check::measure_group(&metadata, &groups, group, &tag)?;
                let declared =
                    semver_check::declares_break(&pull_request, &changes, group, &groups);
                let is_app = group == release_groups::APP_GROUP;
                semver_check::verdict(group, broke, declared, is_app)
            }
        };
        println!("{}", verdict.describe());
        if verdict.failed() {
            failures.push(verdict.describe());
        }
    }

    if failures.is_empty() {
        return Ok(());
    }
    bail!(
        "{count} undeclared API break(s):\n  {list}",
        count = failures.len(),
        list = failures.join("\n  "),
    )
}
