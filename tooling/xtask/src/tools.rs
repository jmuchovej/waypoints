//! The tools xtask talks to, one module each. A module owns every call to
//! its tool, so what it is asked and how its answers are read live in one
//! place.

pub mod devenv;
pub mod git;
pub mod github;
pub mod mise;
pub mod nix;

use anyhow::{Context as _, Result};
use std::process::Command;

/// Runs `program` with `args` and returns its trimmed stdout. A non-zero
/// exit is an error quoting stderr, so a failed call in CI explains itself
/// in the job log.
fn run(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run `{program}` — is it on PATH?"))?;

    if !output.status.success() {
        anyhow::bail!(
            "`{program} {}` failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8(output.stdout)
        .with_context(|| format!("`{program}` emitted invalid UTF-8"))?
        .trim()
        .to_owned())
}
