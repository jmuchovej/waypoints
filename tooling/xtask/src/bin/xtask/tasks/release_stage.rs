//! Renames one platform's published and updater artifacts out of the
//! bundler's nested output into the flat staged directory. See
//! [`xtask::releases::artifacts`]. Touches nothing outside the working tree
//! and resolves no credential.

use anyhow::Result;
use clap::Parser;
use std::path::Path;
use xtask::{Platform, releases::artifacts, workspace};

#[derive(Parser)]
pub struct Options {
    /// The platform whose build output this stages.
    #[arg(long, value_enum)]
    platform: Platform,

    /// The release tag the files are named for. A build-only run has none:
    /// it passes the tag it would have published, or omits this to take
    /// `v` and the app's own version.
    #[arg(long)]
    tag: Option<String>,
}

pub fn run(args: Options) -> Result<()> {
    let tag = match args.tag {
        Some(tag) => tag,
        None => format!("v{}", workspace::version()?),
    };
    for path in artifacts::stage(args.platform, &tag, Path::new("."))? {
        println!("staged {}", path.display());
    }
    Ok(())
}
