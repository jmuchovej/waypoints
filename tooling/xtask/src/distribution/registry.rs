// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `distribution/channels.toml`: which distribution channels each release
//! channel reaches, and the package identity it takes there.

use crate::Channel;
use anyhow::{Context as _, Result};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

/// Where the registry lives, from the repository root.
pub const REGISTRY: &str = "distribution/channels.toml";

/// One table per release channel; a table names a distribution channel and
/// the identity the release channel takes there. A table that is not a
/// release channel is refused.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    #[serde(default)]
    stable: BTreeMap<String, String>,
    #[serde(default)]
    preview: BTreeMap<String, String>,
    #[serde(default)]
    nightly: BTreeMap<String, String>,
}

impl Registry {
    pub fn parse(text: &str) -> Result<Self> {
        toml::from_str(text).context("parsing the distribution registry")
    }

    pub fn read(root: &Path) -> Result<Self> {
        let path = root.join(REGISTRY);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text).with_context(|| path.display().to_string())
    }

    fn table(&self, channel: Channel) -> &BTreeMap<String, String> {
        match channel {
            Channel::Stable => &self.stable,
            Channel::Preview => &self.preview,
            Channel::Nightly => &self.nightly,
        }
    }

    /// The identity `channel` takes on `distribution`, if it is published there.
    pub fn identity(&self, channel: Channel, distribution: &str) -> Option<&str> {
        self.table(channel).get(distribution).map(String::as_str)
    }

    /// Every release channel published to `distribution`, in release order.
    pub fn release_channels(&self, distribution: &str) -> Vec<Channel> {
        [Channel::Stable, Channel::Preview, Channel::Nightly]
            .into_iter()
            .filter(|channel| self.identity(*channel, distribution).is_some())
            .collect()
    }

    /// Every distribution channel some release channel is published to.
    pub fn distribution_channels(&self) -> Vec<&str> {
        let mut all: Vec<&str> = [&self.stable, &self.preview, &self.nightly]
            .into_iter()
            .flat_map(|table| table.keys().map(String::as_str))
            .collect();
        all.sort_unstable();
        all.dedup();
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed() -> Registry {
        Registry::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
    }

    #[test]
    fn the_committed_registry_parses_and_names_every_package_channel() {
        let registry = committed();
        assert_eq!(
            registry.distribution_channels(),
            ["aur", "brew", "chocolatey", "winget"]
        );
        assert_eq!(
            registry.identity(Channel::Nightly, "brew"),
            Some("waypoints@nightly")
        );
        assert_eq!(
            registry.identity(Channel::Preview, "winget"),
            Some("Waypoints.Waypoints.Preview")
        );
    }

    #[test]
    fn a_distribution_channel_lists_the_release_channels_published_to_it() {
        let registry = committed();
        assert_eq!(
            registry.release_channels("brew"),
            [Channel::Stable, Channel::Preview, Channel::Nightly]
        );

        let narrow = Registry::parse("[stable]\nbrew = \"waypoints\"\n").unwrap();
        assert_eq!(narrow.release_channels("brew"), [Channel::Stable]);
        assert_eq!(narrow.identity(Channel::Nightly, "brew"), None);
        assert!(narrow.release_channels("aur").is_empty());
    }

    #[test]
    fn a_table_that_is_not_a_release_channel_is_refused() {
        let error = Registry::parse("[development]\nbrew = \"waypoints@dev\"\n")
            .expect_err("an unknown table is refused");
        assert!(format!("{error:#}").contains("development"), "{error:#}");
    }
}
