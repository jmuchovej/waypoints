//! Records a published tag in the release index on `main`. See
//! [`xtask::releases::index`].

use anyhow::Result;
use clap::Parser;
use xtask::{
    releases::{index, version::Version},
    tools::github,
};

#[derive(Parser)]
pub struct Options {
    /// The tag to record, exactly as created: `v{version}`.
    #[arg(long)]
    tag: String,
    /// Compute and print the entry without committing it.
    #[arg(long)]
    dry_run: bool,
}

pub fn run(args: Options) -> Result<()> {
    let version: Version = args.tag.parse()?;
    let (rev, manifest) = index::tagged_manifest(&args.tag)?;
    let channel = index::classify(&args.tag, &version, &manifest)?;
    println!("{tag} is a {channel} release at {rev}", tag = args.tag);

    let hash = index::source_hash(&rev)?;
    println!("  source: {hash}");

    let entry = index::Entry {
        tag: args.tag.clone(),
        rev,
        hash,
    };
    let path = index::Entry::path(channel);
    let content = entry.render()?;
    println!("{path}:\n{content}");

    if args.dry_run {
        println!("  (dry run — nothing written)");
        return Ok(());
    }

    // The same cross-branch write a cut performs, with the same caveat: if
    // `main` ever becomes a protected branch, this needs a PR path too.
    let commit = github::update_file(&path, "main", &content, &entry.commit_message(channel))?;
    println!("  recorded on main at {commit}");
    Ok(())
}
