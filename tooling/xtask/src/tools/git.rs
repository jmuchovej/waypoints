//! git, read from the checkout.

use anyhow::Result;

/// Runs `git` with `args` and returns its trimmed stdout.
pub fn run(args: &[&str]) -> Result<String> {
    super::run("git", args)
}
