//! A channel's manifest — `{channel}.json` at the bucket root — written once
//! every required target has uploaded successfully. The single commit point
//! a channel advertises through: nothing reads the artifacts this run
//! placed under `{channel}/` except by way of this file, and nothing else
//! in the run writes it. Ordered as the publishing run's last write, after
//! the GitHub release record itself is flipped public, so the reverse (a
//! published record naming artifacts that never arrived) cannot happen.
//!
//! Unlike an artifact upload, this write is a plain overwrite: a channel has
//! exactly one current manifest, and each publish replaces it.

use crate::{Channel, releases::version::Version};
use anyhow::{Context as _, Result};
use serde::Serialize;

/// What a channel's manifest guarantees to a reader: its current version
/// and the targets published for it. Field names are the wire format every
/// reader (the download surface, an updater) agrees on.
#[derive(Serialize)]
pub struct Manifest {
    pub version: String,
    pub series: String,
    pub targets: Vec<ManifestTarget>,
}

#[derive(Serialize)]
pub struct ManifestTarget {
    pub os: String,
    pub arch: String,
    pub extensions: &'static [&'static str],
}

impl Manifest {
    /// Every platform in `channel.required_targets()`, in its published-target
    /// shape. Today that is every platform there is, for every channel, but
    /// the manifest is built from the channel's own required set rather than
    /// a bare enumeration so a channel narrowing it later changes what gets
    /// published without this file needing to change too.
    pub fn build(channel: Channel, version: &Version) -> Self {
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

        Self {
            version: version.tag(),
            series: version.series(),
            targets,
        }
    }

    /// The bytes written to the store.
    pub fn render(&self) -> Result<Vec<u8>> {
        serde_json::to_vec_pretty(self).context("serializing the manifest")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(channel: Channel, tag: &str) -> Manifest {
        Manifest::build(channel, &tag.parse().unwrap())
    }

    #[test]
    fn the_manifest_names_the_tag_and_every_required_target() {
        let manifest = manifest(Channel::Stable, "v0.1.1");
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
        let manifest = manifest(Channel::Nightly, "v0.2.0-nightly.3+2026.10.19");
        let value: serde_json::Value = serde_json::from_slice(&manifest.render().unwrap()).unwrap();
        assert_eq!(value["version"], "v0.2.0-nightly.3+2026.10.19");
        assert_eq!(value["series"], "v0.2.x");
        assert!(value["targets"].as_array().unwrap().iter().any(|target| {
            target["os"] == "macos"
                && target["arch"] == "arm64"
                && target["extensions"] == serde_json::json!(["dmg"])
        }));
    }
}
