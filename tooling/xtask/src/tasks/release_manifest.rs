//! Writes a channel's manifest — `{channel}.json` at the bucket root — once
//! every required target has uploaded successfully. The single commit point
//! a channel advertises through: nothing reads the artifacts this run
//! placed under `{channel}/` except by way of this file, and nothing else
//! in the run writes it. Ordered as the publishing run's last write, after
//! the GitHub release record itself is flipped public, so the reverse (a
//! published record naming artifacts that never arrived) cannot happen.
//!
//! Unlike an artifact upload, this write is a plain overwrite: a channel has
//! exactly one current manifest, and each publish replaces it.

use crate::tasks::{release_plan, release_store, workflows::Channel};
use anyhow::{Context as _, Result};
use cargo_metadata::semver::Version;
use clap::Parser;
use serde::Serialize;

#[derive(Parser)]
pub struct ReleaseManifestArgs {
    /// The release tag this manifest publishes. The channel and series are
    /// derived from it, never supplied — the same rule the uploader follows.
    #[arg(long)]
    tag: String,

    /// How long the derived store credential stays valid, in seconds.
    #[arg(long, default_value_t = release_store::DEFAULT_TTL_SECONDS)]
    ttl_seconds: u64,
}

/// What a channel's manifest guarantees to a reader: its current version
/// and the targets published for it. Field names are the wire format every
/// reader (the download surface, an updater) agrees on.
#[derive(Serialize)]
struct Manifest {
    version: String,
    series: String,
    targets: Vec<ManifestTarget>,
}

#[derive(Serialize)]
struct ManifestTarget {
    os: String,
    arch: String,
    extensions: &'static [&'static str],
}

pub fn run(args: ReleaseManifestArgs) -> Result<()> {
    let version = tag_version(&args.tag)?;
    let channel = release_plan::channel_of(&version);
    let manifest = build_manifest(channel, &args.tag);
    let body = serde_json::to_vec_pretty(&manifest).context("serializing the manifest")?;

    let client = release_store::create_r2_client(channel, args.ttl_seconds)?;
    let key = release_store::manifest_key(channel);

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

/// Strips the tag's leading `v` and parses the rest as semver — the same
/// rule the uploader follows, so the two never derive a different channel
/// for the same tag.
fn tag_version(tag: &str) -> Result<Version> {
    let version = tag
        .strip_prefix('v')
        .with_context(|| format!("tag `{tag}` has no leading `v`"))?;
    Version::parse(version).with_context(|| format!("tag `{tag}` is not a valid version"))
}

/// Every platform in `channel.required_targets()`, in its published-target
/// shape. Today that is every platform there is, for every channel, but the
/// manifest is built from the channel's own required set rather than a bare
/// enumeration so a channel narrowing it later changes what gets published
/// without this file needing to change too.
fn build_manifest(channel: Channel, tag: &str) -> Manifest {
    let series = release_plan::series_of(&tag_version(tag).expect("validated by `run` already"));
    let targets = channel
        .required_targets()
        .iter()
        .copied()
        .map(|platform| {
            let target = platform.published_target();
            ManifestTarget {
                os: target.os,
                arch: target.arch.to_string(),
                extensions: target.extensions,
            }
        })
        .collect();

    Manifest {
        version: tag.to_owned(),
        series,
        targets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_names_the_tag_and_every_required_target() {
        let manifest = build_manifest(Channel::Stable, "v0.1.1");
        assert_eq!(manifest.version, "v0.1.1");
        assert_eq!(manifest.series, "v0.1.x");
        assert_eq!(
            manifest.targets.len(),
            Channel::Stable.required_targets().len()
        );

        let android = manifest
            .targets
            .iter()
            .find(|target| target.os == "android")
            .expect("android target");
        assert_eq!(android.arch, "arm64");
        assert_eq!(android.extensions, &["apk", "aab"]);
    }

    #[test]
    fn the_manifest_serializes_to_the_agreed_wire_shape() {
        let manifest = build_manifest(Channel::Nightly, "v0.2.0-nightly.3+2026.10.19");
        let value = serde_json::to_value(&manifest).unwrap();
        assert_eq!(value["version"], "v0.2.0-nightly.3+2026.10.19");
        assert_eq!(value["series"], "v0.2.x");
        assert!(value["targets"].as_array().unwrap().iter().any(|target| {
            target["os"] == "macos"
                && target["arch"] == "arm64"
                && target["extensions"] == serde_json::json!(["dmg"])
        }));
    }
}
