// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Commits a channel's manifest — the single point a channel advertises
//! through — once every required target has uploaded, or, on a dry run,
//! renders it to a file instead. See [`xtask::releases::manifest`].

use anyhow::{Context as _, Result};
use clap::Parser;
use std::{fs, path::PathBuf};
use xtask::{
    constants::STAGE_DIR,
    releases::version::Version,
    releases::{manifest::Manifest, store},
};

#[derive(Parser)]
pub struct Options {
    /// The release tag this manifest publishes. The channel and series are
    /// derived from it, never supplied — the same rule the uploader follows.
    #[arg(long)]
    tag: String,

    /// The staged directory the upload read; every file it holds for the
    /// tag is hashed into the manifest.
    #[arg(long, default_value = STAGE_DIR)]
    from: PathBuf,

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
    let manifest = Manifest::build(channel, &version, &args.from)?;
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

    /// The rendered file is the body the store would have received, digests
    /// read from the stage included.
    #[test]
    fn a_dry_run_renders_the_manifest_to_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let tag = "v0.2.0-nightly.20261109";
        let staged = dir.path().join(STAGE_DIR);
        fs::create_dir_all(&staged).unwrap();
        let channel = tag.parse::<Version>().unwrap().channel();
        for platform in channel.required_targets() {
            for extension in platform
                .published_target()
                .extensions
                .iter()
                .chain(platform.updater_extensions())
            {
                fs::write(
                    staged.join(xtask::releases::distributables::file_name(tag, extension)),
                    extension,
                )
                .unwrap();
            }
        }

        let into = dir.path().join("dry-run");
        run(Options {
            tag: tag.to_owned(),
            from: staged,
            ttl_seconds: store::DEFAULT_TTL_SECONDS,
            dry_run: true,
            into: Some(into.clone()),
        })
        .unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&fs::read(into.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(written["version"], tag);
        assert_eq!(written["series"], "v0.2.x");
        assert!(
            written["targets"]
                .as_array()
                .unwrap()
                .iter()
                .all(|target| target["digests"].as_object().is_some_and(|d| !d.is_empty()))
        );
    }
}
