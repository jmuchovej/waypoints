//! Queries devenv about what it manages, so no list in this crate has to be
//! kept in sync with `devenv.nix` by hand.

use anyhow::{Context, Result, bail};
use clap::Parser;
use std::{path::Path, process::Command};

#[derive(Parser)]
pub(crate) struct DevenvFilesArgs {
    /// Restrict to files devenv copies into the tree, dropping store symlinks.
    ///
    /// A symlink points into the Nix store and cannot drift, so diffing one is
    /// meaningless. Callers comparing file contents want this; callers that
    /// only need names — a workflow's `paths:` trigger — do not.
    #[arg(long)]
    copied: bool,
}

/// Prints one path per line, so a caller can consume it with a plain `for`.
pub(crate) fn run(args: DevenvFilesArgs) -> Result<()> {
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
pub(crate) fn copied_files() -> Result<Vec<String>> {
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
pub(crate) fn managed_files() -> Result<Vec<String>> {
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
