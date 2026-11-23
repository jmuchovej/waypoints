// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! What a package manifest's placeholders read: the identity from the
//! registry, the flavor's identity from its Tauri config, and the release and
//! its artifacts from the release manifest.
//!
//! - `package.id`
//! - `flavor.product_name`, `flavor.identifier`, `flavor.executable`
//! - `release.channel`, `release.tag`, `release.version` (the tag without
//!   `v`), `release.series`
//! - `artifact.<os>_<arch>.<ext>.{url, file_name, sha256}`, with any `.` in
//!   the extension spelled `_` (`app_tar_gz`)
//!
//! and the filter `pkgver`, for the AUR's hyphen-free version.

use crate::{
    Channel, Flavor,
    releases::{artifacts, manifest::Manifest},
};
use anyhow::{Context as _, Result, anyhow};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::path::Path;

/// Where the download surface serves the store's objects, by key.
pub const DOWNLOADS: &str = "https://waypoints.so/api/v1/releases";

/// The fields of a flavor's Tauri config a package manifest names.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlavorConfig {
    product_name: String,
    identifier: String,
}

fn flavor_config(root: &Path, channel: Channel) -> Result<FlavorConfig> {
    let path = root.join(format!(
        "src-tauri/res/configs/flavors/{}.toml",
        channel.flavor()
    ));
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Arch's `pkgver` forbids a hyphen: `v0.1.0-pre.2` → `0.1.0pre.2`.
pub fn pkgver(tag: &str) -> String {
    tag.strip_prefix('v').unwrap_or(tag).replace('-', "")
}

/// The `pkgver` filter, over a string value.
pub fn pkgver_filter(tag: &str, _: tera::Kwargs, _: &tera::State) -> String {
    pkgver(tag)
}

/// The render context for `channel` published as `package_id`, from
/// `manifest`. Every artifact the manifest lists must carry a digest.
pub fn build(
    root: &Path,
    channel: Channel,
    package_id: &str,
    manifest: &Manifest,
) -> Result<tera::Context> {
    let value = value(root, channel, package_id, manifest)?;
    tera::Context::from_serialize(&value).context("building the render context")
}

/// [`build`]'s context as plain JSON.
fn value(root: &Path, channel: Channel, package_id: &str, manifest: &Manifest) -> Result<Value> {
    let config = flavor_config(root, channel)?;
    let tag = &manifest.version;

    let mut artifacts = Map::new();
    for target in &manifest.targets {
        let published = channel
            .required_targets()
            .iter()
            .map(|platform| platform.published_target())
            .find(|published| {
                published.os == target.os && published.arch.to_string() == target.arch
            })
            .ok_or_else(|| {
                anyhow!(
                    "the release manifest names {}/{}, which {channel} does not publish",
                    target.os,
                    target.arch
                )
            })?;

        let mut extensions = Map::new();
        let listed = target.extensions.iter().chain(
            target
                .digests
                .keys()
                .filter(|extension| !target.extensions.contains(extension)),
        );
        for extension in listed {
            let file_name = artifacts::file_name(tag, extension);
            let sha256 = target
                .digests
                .get(extension)
                .ok_or_else(|| anyhow!("the release manifest records no digest for {file_name}"))?;
            let key = artifacts::object_key(channel, &manifest.series, &published, tag, extension);
            extensions.insert(
                extension.replace('.', "_"),
                json!({
                    "url": format!("{DOWNLOADS}/{key}"),
                    "file_name": file_name,
                    "sha256": sha256,
                }),
            );
        }
        artifacts.insert(
            format!("{}_{}", target.os, target.arch),
            Value::Object(extensions),
        );
    }

    Ok(json!({
        "package": { "id": package_id },
        "flavor": {
            "product_name": config.product_name,
            "identifier": config.identifier,
            "executable": Flavor::from(channel).executable(),
        },
        "release": {
            "channel": channel.flavor(),
            "tag": tag,
            "version": tag.strip_prefix('v').unwrap_or(tag),
            "series": manifest.series,
        },
        "artifact": artifacts,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    fn fixture() -> Manifest {
        serde_json::from_str(include_str!("../../tests/fixtures/release-manifest.json")).unwrap()
    }

    #[test]
    fn pkgver_drops_the_v_and_every_hyphen() {
        assert_eq!(pkgver("v0.1.0-pre.2"), "0.1.0pre.2");
        assert_eq!(pkgver("v0.1.0-nightly.20261116"), "0.1.0nightly.20261116");
        assert_eq!(pkgver("v0.1.1"), "0.1.1");
    }

    #[test]
    fn the_context_names_the_package_flavor_release_and_artifacts() {
        let value = value(root(), Channel::Nightly, "waypoints@nightly", &fixture()).unwrap();
        assert_eq!(value["package"]["id"], "waypoints@nightly");
        assert_eq!(value["flavor"]["product_name"], "Waypoints Nightly");
        assert_eq!(value["flavor"]["identifier"], "so.waypoints.nightly");
        assert_eq!(value["flavor"]["executable"], "waypoints-nightly");
        assert_eq!(value["release"]["channel"], "nightly");
        assert_eq!(value["release"]["tag"], "v0.1.0-nightly.20261116");
        assert_eq!(value["release"]["version"], "0.1.0-nightly.20261116");
        assert_eq!(value["release"]["series"], "v0.1.x");

        let dmg = &value["artifact"]["macos_arm64"]["dmg"];
        assert_eq!(dmg["file_name"], "waypoints-v0.1.0-nightly.20261116.dmg");
        assert_eq!(
            dmg["url"],
            "https://waypoints.so/api/v1/releases/nightly/v0.1.x/macos/arm64/waypoints-v0.1.0-nightly.20261116.dmg"
        );
        assert_eq!(dmg["sha256"].as_str().unwrap().len(), 64);
        assert!(value["artifact"]["macos_arm64"]["app_tar_gz_sig"].is_object());
        assert!(value["artifact"]["linux_amd64"]["AppImage"].is_object());
    }

    #[test]
    fn a_missing_digest_fails_naming_the_artifact() {
        let mut manifest = fixture();
        let linux = manifest
            .targets
            .iter_mut()
            .find(|target| target.os == "linux")
            .unwrap();
        linux.digests.remove("deb");
        let error = build(root(), Channel::Nightly, "waypoints-nightly-bin", &manifest)
            .expect_err("a missing digest is refused");
        assert!(
            format!("{error:#}").contains("waypoints-v0.1.0-nightly.20261116.deb"),
            "{error:#}"
        );
    }
}
