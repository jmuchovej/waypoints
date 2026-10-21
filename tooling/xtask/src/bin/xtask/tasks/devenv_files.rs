use anyhow::Result;
use clap::Parser;
use xtask::tools::devenv;

#[derive(Parser)]
pub struct Options {
    /// Restrict to files devenv copies into the tree, dropping store symlinks.
    /// Wanted by callers that diff contents; a symlink into the store cannot
    /// drift.
    #[arg(long)]
    copied: bool,
}

/// Prints one path per line, so a caller can consume it with a plain `for`.
pub fn run(args: Options) -> Result<()> {
    let files = if args.copied {
        devenv::copied_files()?
    } else {
        devenv::managed_files()?
    };

    for file in files {
        println!("{file}");
    }

    Ok(())
}
