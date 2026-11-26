// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The channels a release ships through, and the flavors the app is built
//! as — the same set plus a development flavor that never ships.

use crate::Platform;
use derive_more::{Display, IsVariant};
use strum::EnumIter;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Display,
    IsVariant,
    clap::ValueEnum,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[display(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Preview,
    Nightly,
}

impl Channel {
    /// One string naming the workflow, the `just icons::<flavor>` recipe, and
    /// the cargo feature: this channel's [`Flavor`].
    pub fn flavor(self) -> String {
        Flavor::from(self).to_string()
    }

    pub fn prerelease(self) -> bool {
        !self.is_stable()
    }

    /// Platforms that must each upload successfully before this channel's
    /// manifest is written. All three channels require today's full build
    /// matrix — see the design's "Targets are the host architecture of each
    /// platform" decision; a channel narrowing its own required set is a
    /// future change to this, not to the publish gate that reads it.
    pub fn required_targets(self) -> &'static [Platform] {
        use Platform::*;
        &[macOS, Windows, Linux, iOS, Android]
    }

    /// The tag pattern a push must match to trigger this channel.
    /// `None` for nightly, which mints its own tags — a push trigger would
    /// re-enter the workflow on the tag it had just created.
    pub fn tag_pattern(self) -> Option<&'static str> {
        use Channel::*;
        match self {
            Stable => Some("v[0-9]+.[0-9]+.[0-9]+"),
            Preview => Some("v[0-9]+.[0-9]+.[0-9]+-pre.[0-9]+"),
            Nightly => None,
        }
    }
}

/// Every flavor the app is built as: each release [`Channel`], plus the
/// development build. One name per flavor, shared by the `just
/// icons::<flavor>` recipe, the cargo feature, the Typst icon render, and
/// the release workflow.
///
/// A superset type rather than a `Channel` variant: a channel is something
/// a release ships through, and nothing that iterates channels — required
/// targets, tag patterns, the publish gate — should ever meet the
/// development build. Convert a channel with `Flavor::from`.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Display,
    EnumIter,
    clap::ValueEnum,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[display(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Flavor {
    Development,
    Nightly,
    Preview,
    Stable,
}

impl From<Channel> for Flavor {
    fn from(channel: Channel) -> Self {
        match channel {
            Channel::Stable => Flavor::Stable,
            Channel::Preview => Flavor::Preview,
            Channel::Nightly => Flavor::Nightly,
        }
    }
}

impl Flavor {
    /// The binary this flavor builds — a `[[bin]]` in `src-tauri/Cargo.toml`.
    pub fn executable(self) -> &'static str {
        match self {
            Flavor::Stable => "waypoints",
            Flavor::Preview => "waypoints-preview",
            Flavor::Nightly => "waypoints-nightly",
            Flavor::Development => "waypoints-development",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator as _;

    /// Today's required set is every platform, for every channel — a channel
    /// narrowing it is a change to this list, not to the gate that counts
    /// against it.
    #[test]
    fn every_channel_requires_the_full_target_set() {
        let all: Vec<Platform> = Platform::iter().collect();
        for channel in [Channel::Stable, Channel::Preview, Channel::Nightly] {
            assert_eq!(channel.required_targets(), all.as_slice(), "{channel}");
        }
    }

    /// Every channel is a flavor under the same name, and the development
    /// flavor is the one that is no channel.
    #[test]
    fn every_channel_is_a_flavor_and_development_is_not_a_channel() {
        for channel in [Channel::Nightly, Channel::Preview, Channel::Stable] {
            assert_eq!(Flavor::from(channel).to_string(), channel.to_string());
            assert_eq!(channel.flavor(), channel.to_string());
        }
        let flavors: Vec<Flavor> = Flavor::iter().collect();
        assert_eq!(flavors.len(), 4);
        assert!(flavors.contains(&Flavor::Development));
        assert_eq!(Flavor::Development.to_string(), "development");
    }

    /// Each flavor's executable is the `[[bin]]` that requires its feature.
    #[test]
    fn every_executable_is_the_app_manifest_s_bin() {
        #[derive(serde::Deserialize)]
        struct Manifest {
            bin: Vec<Bin>,
        }
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct Bin {
            name: String,
            required_features: Vec<String>,
        }

        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/Cargo.toml");
        let manifest: Manifest = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for flavor in Flavor::iter() {
            let bin = manifest
                .bin
                .iter()
                .find(|bin| bin.required_features.contains(&flavor.to_string()))
                .unwrap_or_else(|| panic!("no [[bin]] requires `{flavor}`"));
            assert_eq!(flavor.executable(), bin.name, "{flavor}");
        }
    }
}
