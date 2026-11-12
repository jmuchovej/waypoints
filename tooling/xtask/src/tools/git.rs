// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! git, read from the checkout.

use anyhow::{Context as _, Result, bail};
use std::process::Command;

/// Runs `git` with `args` and returns its trimmed stdout.
pub fn run(args: &[&str]) -> Result<String> {
    super::run("git", args)
}

/// The file at `path` in the tree `reference` names, byte for byte: untrimmed,
/// so a file's final newline survives into a diff of it.
pub fn show(reference: &str, path: &str) -> Result<String> {
    let spec = format!("{reference}:{path}");
    let output = Command::new("git")
        .args(["show", &spec])
        .output()
        .with_context(|| format!("running `git show {spec}`"))?;
    if !output.status.success() {
        bail!(
            "`git show {spec}` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).with_context(|| format!("{spec} is not UTF-8"))
}

/// Every `v*` tag as `origin` holds it: what is published, rather than what
/// this checkout happens to have fetched or kept. Fails when the remote
/// cannot be reached, since deciding from local tags would be deciding from
/// stale ones.
pub fn remote_tags() -> Result<Vec<String>> {
    let listed = run(&["ls-remote", "--tags", "--refs", "origin", "v*"])
        .context("listing origin's tags: a planner decides from what is published")?;
    Ok(tag_names(&listed))
}

/// The tag names in `git ls-remote --tags --refs` output: one `<sha>\t<ref>`
/// line per tag.
fn tag_names(listed: &str) -> Vec<String> {
    listed
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .filter_map(|reference| reference.strip_prefix("refs/tags/"))
        .map(str::to_owned)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `ls-remote` prints `<sha>\t<ref>`; only the tag names matter, and a
    /// line that is not a tag ref contributes nothing.
    #[test]
    fn the_listing_yields_tag_names() {
        let listed = "48f4fef5c0000000000000000000000000000000\trefs/tags/v0.1.0-nightly.20261025\n\
                      4444c8a593bf57e16e0ddca8fd05cdbbf7106c19\trefs/tags/v0.1.0-nightly.20261108\n\
                      0000000000000000000000000000000000000000\trefs/heads/v0.1.x\n";
        assert_eq!(
            tag_names(listed),
            ["v0.1.0-nightly.20261025", "v0.1.0-nightly.20261108"]
        );
        assert!(tag_names("").is_empty());
    }
}
