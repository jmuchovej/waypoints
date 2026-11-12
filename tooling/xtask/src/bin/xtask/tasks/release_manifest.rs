// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Writes a channel's manifest once every required target has uploaded — or,
//! on a dry run, renders it to a file instead. See
//! [`xtask::releases::manifest`].

use anyhow::{Context as _, Result};
use clap::Parser;
use std::{fs, path::PathBuf};
use xtask::{
    releases::version::Version,
    releases::{manifest::Manifest, store},
};

#[derive(Parser)]
pub struct Options {
    /// The release tag this manifest publishes. The channel and series are
    /// derived from it, never supplied — the same rule the uploader follows.
    #[arg(long)]
    tag: String,

    /// How long the derived store credential stays valid, in seconds.
    #[arg(long, default_value_t = store::DEFAULT_TTL_SECONDS)]
    ttl_seconds: u64,

    /// Render the manifest under `--into` instead of writing the store. No
    /// store client is built, so no credential is derived.
    #[arg(long, requires = "into")]
    dry_run: bool,

    /// The directory a dry run writes `manifest.json` under.
    #[arg(long, requires = "dry_run")]
    into: Option<PathBuf>,
}

pub fn run(args: Options) -> Result<()> {
    let version: Version = args.tag.parse()?;
    let channel = version.channel();
    let manifest = Manifest::build(channel, &version);
    let body = manifest.render()?;

    if args.dry_run {
        let dir = args.into.context("`--dry-run` needs `--into`")?;
        let path = dir.join("manifest.json");
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        fs::write(&path, &body).with_context(|| format!("writing {}", path.display()))?;
        println!(
            "dry run: rendered {} naming {}",
            path.display(),
            manifest.version
        );
        return Ok(());
    }

    let client = store::create_r2_client(channel, args.ttl_seconds)?;
    let key = store::manifest_key(channel);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;
    runtime.block_on(async {
        client
            .write(&key, body)
            .await
            .with_context(|| format!("writing the manifest to {key}"))
    })?;

    println!("wrote {key} naming {}", manifest.version);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dry run names where it renders, and a real write names no directory.
    #[test]
    fn a_dry_run_needs_a_directory_and_a_real_write_takes_none() {
        let dry = Options::try_parse_from([
            "release-manifest",
            "--tag",
            "v0.1.1",
            "--dry-run",
            "--into",
            "out",
        ])
        .unwrap();
        assert!(dry.dry_run);
        assert_eq!(dry.into.as_deref(), Some(std::path::Path::new("out")));
        assert!(
            Options::try_parse_from(["release-manifest", "--tag", "v0.1.1", "--dry-run"]).is_err()
        );
        assert!(
            Options::try_parse_from(["release-manifest", "--tag", "v0.1.1", "--into", "out"])
                .is_err()
        );
        let real = Options::try_parse_from(["release-manifest", "--tag", "v0.1.1"]).unwrap();
        assert!(!real.dry_run && real.into.is_none());
    }

    /// The rendered file is the body the store would have received.
    #[test]
    fn a_dry_run_renders_the_manifest_to_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let into = dir.path().join("dry-run");
        run(Options {
            tag: "v0.2.0-nightly.20261109".to_owned(),
            ttl_seconds: store::DEFAULT_TTL_SECONDS,
            dry_run: true,
            into: Some(into.clone()),
        })
        .unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&fs::read(into.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(written["version"], "v0.2.0-nightly.20261109");
        assert_eq!(written["series"], "v0.2.x");
    }
}
