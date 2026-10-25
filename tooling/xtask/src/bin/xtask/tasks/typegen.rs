//! Regenerates the TypeScript bindings from the Rust data model. The exports
//! are [`xtask::typegen`]'s; this decides which crates' files to write, writes
//! them, and runs `treefmt` over them last so the tree matches what a commit
//! holds. `just check-generated` regenerates these and diffs them.

use anyhow::{Context as _, Result, bail};
use clap::{Parser, ValueEnum};
use std::{fs, path::PathBuf, process::Command};
use xtask::{typegen, workspace};

#[derive(Parser)]
pub struct Options {
    /// Write one crate's bindings only. Domain's export is still computed when
    /// cartograph is chosen — the rewrite needs its names — just not written.
    #[arg(long, value_enum)]
    only: Option<Crate>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Crate {
    Domain,
    Cartograph,
}

pub fn run(args: Options) -> Result<()> {
    let metadata = workspace::load()?;
    let typescript_dir = |package: &str| -> Result<PathBuf> {
        metadata
            .packages
            .iter()
            .find(|candidate| candidate.name.as_str() == package)
            .and_then(|candidate| candidate.manifest_path.parent())
            .map(|crate_dir| crate_dir.join("typescript").into_std_path_buf())
            .with_context(|| format!("no `{package}` package in this workspace"))
    };

    let exports = typegen::exports()?;

    let mut written = Vec::new();
    if args.only != Some(Crate::Cartograph) {
        let dir = typescript_dir("domain")?;
        written.push(write(dir.join("types.g.ts"), &exports.domain)?);
        written.push(write(dir.join("schema.g.ts"), &exports.schema)?);
    }
    if args.only != Some(Crate::Domain) {
        let dir = typescript_dir("cartograph")?;
        written.push(write(dir.join("types.g.ts"), &exports.cartograph)?);
    }

    treefmt(&written)?;
    for path in &written {
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn write(path: PathBuf, contents: &str) -> Result<PathBuf> {
    fs::write(&path, contents).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(path)
}

/// Formats exactly the files this task wrote. The committed files are
/// formatted, so the lockstep diff is only honest if the regenerated ones are.
fn treefmt(paths: &[PathBuf]) -> Result<()> {
    let status = Command::new("treefmt")
        .args(paths)
        .status()
        .context("Failed to run `treefmt` — is treefmt on PATH?")?;

    if !status.success() {
        bail!("`treefmt` failed with {status}");
    }
    Ok(())
}
