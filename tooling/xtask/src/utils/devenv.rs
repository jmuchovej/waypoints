//! Queries devenv about what it manages, so no list in this crate has to be
//! kept in sync with `devenv.nix` by hand.

use anyhow::{Context, Result, bail};
use clap::Parser;
use std::{path::Path, process::Command};

#[derive(Parser)]
pub struct DevenvFilesArgs {
    /// Restrict to files devenv copies into the tree, dropping store symlinks.
    ///
    /// A symlink points into the Nix store and cannot drift, so diffing one is
    /// meaningless. Callers comparing file contents want this; callers that
    /// only need names — a workflow's `paths:` trigger — do not.
    #[arg(long)]
    copied: bool,
}

/// Prints one path per line, so a caller can consume it with a plain `for`.
pub fn run(args: DevenvFilesArgs) -> Result<()> {
    let files = if args.copied {
        copied_files()?
    } else {
        managed_files()?
    };

    for file in files {
        println!("{file}");
    }

    Ok(())
}

/// [`managed_files`] minus anything symlinked into the store.
///
/// Unlike `managed_files`, this touches the filesystem — so it must never feed
/// anything baked into a generated file, where the answer would depend on
/// whether the devenv shell had been entered yet.
pub fn copied_files() -> Result<Vec<String>> {
    Ok(managed_files()?
        .into_iter()
        .filter(|file| !Path::new(file).is_symlink())
        .collect())
}

/// Files devenv materializes into the working tree, straight from the one
/// place that knows.
///
/// Deliberately unfiltered. `just check-generated` drops store symlinks before
/// diffing them, because a symlink cannot drift — but a path trigger has no
/// such need: a symlinked file is gitignored, so it can never appear in a diff
/// anyway. Probing the filesystem here would instead make generated output
/// depend on whether the caller had entered the shell yet.
pub fn managed_files() -> Result<Vec<String>> {
    let output = Command::new("devenv")
        .arg("info")
        .output()
        .context("Failed to run `devenv info` — is devenv on PATH?")?;

    if !output.status.success() {
        bail!(
            "`devenv info` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8(output.stdout).context("`devenv info` emitted invalid UTF-8")?;

    let mut files: Vec<String> = stdout
        .lines()
        .skip_while(|line| line.trim() != "# files")
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .filter_map(|line| line.trim().strip_prefix("- "))
        .map(str::to_owned)
        .collect();

    // An empty list means the section vanished or was renamed. Fail loudly:
    // silently emitting a workflow that watches nothing is the one outcome
    // worse than not generating at all.
    if files.is_empty() {
        bail!("`devenv info` listed no managed files. Has its output format changed?");
    }

    files.sort();
    Ok(files)
}

/// devenv profile holding the Android toolchain. Android tools live in an
/// opt-in profile rather than the default shell, which is why they never reach
/// `mise.toml` and CI has to be told about them explicitly.
const ANDROID_PROFILE: &str = "android";

/// The Android NDK version devenv installs, so CI provisions the same one.
///
/// devenv defaults this to a specific version rather than "latest", and the
/// local shell hook resolves it by listing `$ANDROID_HOME/ndk` — CI has no such
/// directory to list, so it has to be told. Asking devenv keeps the two from
/// drifting.
pub fn android_ndk_version() -> Result<String> {
    let attribute = "android.ndk.version";
    let versions = eval(Some(ANDROID_PROFILE), attribute)?;

    // The option is a list; devenv installs each one. CI builds against a
    // single NDK, so it takes the first — the same one the local shell hook
    // lands on.
    versions
        .get(0)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{attribute} was not a non-empty list: {versions}"))
}

/// The JDK major version devenv installs, for `setup-java`.
///
/// Only the major is used. devenv pins an exact patch (21.0.11), but
/// `setup-java` resolves against Zulu's own release list, where an exact patch
/// may simply not exist — so the major is the strongest claim both can honour.
pub fn android_jdk_major() -> Result<String> {
    let attribute = "languages.java.jdk.package.version";
    let version = eval(Some(ANDROID_PROFILE), attribute)?;

    version
        .as_str()
        .and_then(|version| version.split('.').next())
        .map(str::to_owned)
        .with_context(|| format!("{attribute} was not a version string: {version}"))
}

/// Runs `devenv eval <attribute>` and returns the attribute's value.
fn eval(profile: Option<&str>, attribute: &str) -> Result<serde_json::Value> {
    let mut command = Command::new("devenv");
    if let Some(profile) = profile {
        command.args(["--profile", profile]);
    }

    let output = command
        .args(["eval", attribute])
        .output()
        .context("Failed to run `devenv` — is devenv on PATH?")?;

    if !output.status.success() {
        bail!(
            "`devenv eval {attribute}` failed:\n{}",
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
