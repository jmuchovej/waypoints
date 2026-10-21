//! nix, for what only its fetchers can answer.

use anyhow::Result;

/// Runs `nix` with `args` and returns its trimmed stdout.
pub fn run(args: &[&str]) -> Result<String> {
    super::run("nix", args)
}
