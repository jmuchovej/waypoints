use std::process::Command;

use anyhow::{Context as _, Result, bail};
use indoc::indoc;
use serde_json::Value as Json;

pub fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to run `git`")?;

    if !output.status.success() {
        anyhow::bail!(
            "`git {}` failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8(output.stdout)
        .context("`git` emitted invalid UTF-8")?
        .trim()
        .to_owned())
}

/// Satisfies secretspec's `require_reason` policy, which gates every devenv
/// evaluation behind a stated purpose.
const DEVENV_SECRETSPEC_REASON: &str = indoc! {"
  xtask reads devenv's managed-file list and Android toolchain versions to
  generate CI workflows; no secret values are read
"};

/// Runs `devenv eval <attribute>` and returns the attribute's value.
pub fn devenv(attribute: &str) -> Result<Json> {
    let output = Command::new("devenv")
        .args(["eval", attribute])
        .env("SECRETSPEC_REASON", DEVENV_SECRETSPEC_REASON)
        .output()
        .context("Failed to run `devenv` — is devenv on PATH?")?;

    if !output.status.success() {
        bail!(
            "`devenv {attribute}` failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    // `devenv eval` answers with a single-entry object keyed by the attribute.
    let mut evaluated: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("`devenv eval` did not emit JSON")?;

    evaluated
        .get_mut(attribute)
        .map(serde_json::Value::take)
        .with_context(|| format!("`devenv eval` did not return {attribute}"))
}

pub fn gh(args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("Failed to run `gh`")?;

    if !output.status.success() {
        anyhow::bail!(
            "`gh {}` failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8(output.stdout)
        .context("`gh` emitted invalid UTF-8")?
        .trim()
        .to_owned())
}

pub fn env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("${name} is not set"))
}
