// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! git, read from the checkout.

use anyhow::Result;

/// Runs `git` with `args` and returns its trimmed stdout.
pub fn run(args: &[&str]) -> Result<String> {
    super::run("git", args)
}

/// Puts the working tree on `branch` as the remote has it.
///
/// Fetched first because a cut creates its branch after the job checked out,
/// and `main` can move while a job runs. A named branch rather than a detached
/// HEAD, because the pull-request step takes the checked-out branch as its base
/// and refuses to guess from a commit.
pub fn check_out_latest(branch: &str) -> Result<()> {
    let remote = format!("refs/remotes/origin/{branch}");
    run(&["fetch", "origin", &format!("+refs/heads/{branch}:{remote}")])?;
    run(&["checkout", "-B", branch, &remote])?;
    Ok(())
}
