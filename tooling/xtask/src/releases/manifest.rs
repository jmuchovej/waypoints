// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A channel's manifest — `{channel}.json` at the bucket root — written once
//! every required target has uploaded successfully. The single commit point
//! a channel advertises through: nothing reads the artifacts this run
//! placed under `{channel}/` except by way of this file, and nothing else
//! in the run writes it. Ordered as the publishing run's last write, after
//! the GitHub release record itself is flipped public, so the reverse (a
//! published record naming artifacts that never arrived) cannot happen.
//!
//! Every file a target uploads is recorded with its SHA-256, hashed from the
//! staged file the upload read, so a package manifest derives its checksums
//! from this record without a store credential or a re-download.
//!
//! Unlike an artifact upload, this write is a plain overwrite: a channel has
//! exactly one current manifest, and each publish replaces it.

use crate::{
    Channel,
    releases::{artifacts, version::Version},
};
use anyhow::{Context as _, Result};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::{collections::BTreeMap, path::Path};

/// What a channel's manifest guarantees to a reader: its current version
/// and the targets published for it. Field names are the wire format every
/// reader (the download surface, an updater, a package manifest) agrees on.
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
    /// Extension → SHA-256 hex, for every file the target uploads:
    /// installers and updater payloads and signatures alike.
    pub digests: BTreeMap<String, String>,
}

impl Manifest {
    /// Every platform in `channel.required_targets()`, in its published-target
    /// shape, with each uploaded file's digest read from `staged` — the flat
    /// directory `release-upload --from` reads. Today that is every platform
    /// there is, for every channel, but the manifest is built from the
    /// channel's own required set rather than a bare enumeration so a channel
    /// narrowing it later changes what gets published without this file
    /// needing to change too.
    pub fn build(channel: Channel, version: &Version, staged: &Path) -> Result<Self> {
        let tag = version.tag();
        let targets = channel
            .required_targets()
            .iter()
            .copied()
            .map(|platform| {
                let target = platform.published_target();
                let digests = target
                    .extensions
                    .iter()
                    .chain(platform.updater_extensions())
                    .map(|extension| {
                        let path = staged.join(artifacts::file_name(&tag, extension));
                        let bytes = std::fs::read(&path)
                            .with_context(|| format!("hashing {}", path.display()))?;
                        Ok(((*extension).to_owned(), hex::encode(Sha256::digest(&bytes))))
                    })
                    .collect::<Result<_>>()?;
                Ok(ManifestTarget {
                    os: target.os,
                    arch: target.arch.to_string(),
                    extensions: target.extensions,
                    digests,
                })
            })
            .collect::<Result<_>>()?;

        Ok(Self {
            version: tag,
            series: version.series(),
            targets,
        })
    }

    /// The bytes written to the store.
    pub fn render(&self) -> Result<Vec<u8>> {
        serde_json::to_vec_pretty(self).context("serializing the manifest")
    }
}

#[cfg(test)]
pub(crate) mod fixture {
    use crate::{Channel, releases::artifacts};
    use std::path::Path;

    /// The bytes a scratch stage holds for an extension — distinct per file,
    /// so a digest filed under the wrong extension is caught.
    pub fn contents(extension: &str) -> Vec<u8> {
        format!("staged {extension}").into_bytes()
    }

    /// Fills `dir` with every file `channel` publishes for `tag`, as
    /// `release-stage` would have.
    pub fn stage(dir: &Path, channel: Channel, tag: &str) {
        for platform in channel.required_targets() {
            let target = platform.published_target();
            for extension in target
                .extensions
                .iter()
                .chain(platform.updater_extensions())
            {
                std::fs::write(
                    dir.join(artifacts::file_name(tag, extension)),
                    contents(extension),
                )
                .unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Platform;

    fn manifest(channel: Channel, tag: &str) -> Manifest {
        let dir = tempfile::tempdir().unwrap();
        fixture::stage(dir.path(), channel, tag);
        Manifest::build(channel, &tag.parse().unwrap(), dir.path()).unwrap()
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
        let manifest = manifest(Channel::Nightly, "v0.2.0-nightly.20261019");
        let value: serde_json::Value = serde_json::from_slice(&manifest.render().unwrap()).unwrap();
        assert_eq!(value["version"], "v0.2.0-nightly.20261019");
        assert_eq!(value["series"], "v0.2.x");
        assert!(value["targets"].as_array().unwrap().iter().any(|target| {
            target["os"] == "macos"
                && target["arch"] == "arm64"
                && target["extensions"] == serde_json::json!(["dmg"])
                && target["digests"]["dmg"].is_string()
        }));
    }

    #[test]
    fn every_uploaded_file_of_a_target_has_its_own_digest() {
        let manifest = manifest(Channel::Nightly, "v0.2.0-nightly.20261019");
        for (platform, target) in Channel::Nightly
            .required_targets()
            .iter()
            .zip(&manifest.targets)
        {
            let expected: Vec<&str> = target
                .extensions
                .iter()
                .chain(platform.updater_extensions())
                .copied()
                .collect();
            assert_eq!(
                target
                    .digests
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                {
                    let mut sorted = expected.clone();
                    sorted.sort_unstable();
                    sorted
                },
                "{platform}"
            );
            for extension in expected {
                assert_eq!(
                    target.digests[extension],
                    hex::encode(Sha256::digest(fixture::contents(extension))),
                    "{platform} {extension}"
                );
            }
        }
        let macos = Channel::Nightly
            .required_targets()
            .iter()
            .position(|platform| *platform == Platform::macOS)
            .expect("macOS target");
        assert!(
            manifest.targets[macos]
                .digests
                .contains_key("app.tar.gz.sig")
        );
    }

    #[test]
    fn a_file_missing_from_the_stage_fails_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let tag = "v0.1.1";
        fixture::stage(dir.path(), Channel::Stable, tag);
        std::fs::remove_file(dir.path().join(artifacts::file_name(tag, "deb.sig"))).unwrap();
        let error = Manifest::build(Channel::Stable, &tag.parse().unwrap(), dir.path())
            .err()
            .expect("a missing file is refused");
        assert!(
            format!("{error:#}").contains("waypoints-v0.1.1.deb.sig"),
            "{error:#}"
        );
    }
}
