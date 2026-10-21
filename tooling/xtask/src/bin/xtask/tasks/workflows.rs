//! Regenerates every checked-in GitHub Actions workflow. See
//! [`xtask::workflows`].

use anyhow::Result;
use clap::Parser;
use derive_more::AsRef;
use xtask::workflows;

#[derive(Parser)]
pub struct Options {
    /// Report drift instead of writing. Exits non-zero when a generated
    /// workflow on disk differs from what this crate would emit.
    #[arg(long)]
    pub check: bool,

    /// Pin generated workflows to a specific commit, for workflows shipped to
    /// repositories that consume this one.
    #[arg(value_parser = parse_sha)]
    pub sha: Option<GitSHA>,
}

pub fn run(args: Options) -> Result<()> {
    workflows::generate(args.check)
}

#[derive(Clone, AsRef)]
#[as_ref(str)]
pub struct GitSHA(String);

fn parse_sha(value: &str) -> Result<GitSHA, String> {
    const GIT_SHA_LENGTH: usize = 40;
    (value.len() == GIT_SHA_LENGTH)
        .then_some(value)
        .ok_or_else(|| {
            format!(
                "Git SHA has the wrong length! Found {len} chars, expected {GIT_SHA_LENGTH} chars.",
                len = value.len()
            )
        })
        .and_then(|value| {
            let mut tmp = [0; 4];
            value
                .chars()
                .all(|c| u16::from_str_radix(c.encode_utf8(&mut tmp), 16).is_ok())
                .then_some(value)
                .ok_or_else(|| "Not a valid Git SHA".to_owned())
        })
        .and_then(|sha| {
            std::process::Command::new("git")
                .args([
                    "rev-parse",
                    "--quiet",
                    "--verify",
                    &format!("{sha}^{{commit}}"),
                ])
                .output()
                .map_err(|_| "Failed to spawn `git` to verify SHA".to_owned())
                .and_then(|output| {
                    output
                        .status
                        .success()
                        .then_some(sha)
                        .ok_or_else(|| format!("SHA {sha} isn't a valid Git SHA in this repo!"))
                })
        })
        .map(|sha| GitSHA(sha.to_owned()))
}
