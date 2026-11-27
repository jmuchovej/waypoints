// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Where a build's library group versions come from, in order: a value
//! injected into the build, `git describe` against the group's tags, and the
//! version the group file declares (library-minting design D6). Every branch
//! builds its libraries in-tree, so on a series the description runs from the
//! tag the branch descends from, and a cherry-picked fix reads as commits
//! past it; a shallow clone never fails to compile, and never claims more
//! than the file says.
//!
//! Shared between `build.rs`, which bakes the answer into the binary, and the
//! crate, which reads it back — so the resolution can be held to its order
//! by the crate's tests. Standard library only, for the build script's sake.

use std::collections::BTreeMap;

/// The variable a build takes injected versions from, and bakes the resolved
/// ones into: `bedrock=bedrock-v0.4.0,domain=domain-v0.2.1,…`.
pub const ENV: &str = "WAYPOINTS_GROUP_VERSIONS";

/// One library group, as the resolution needs it.
pub struct Group<'a> {
    pub name: &'a str,
    /// The part of the group's tag template before its version.
    pub prefix: &'a str,
    /// What the group file declares.
    pub declared: &'a str,
}

/// The injected versions, `group=version` pairs separated by commas; a pair
/// without `=` is skipped.
pub fn parse(rendered: &str) -> BTreeMap<String, String> {
    rendered
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(group, version)| (group.trim().to_owned(), version.trim().to_owned()))
        .filter(|(group, version)| !group.is_empty() && !version.is_empty())
        .collect()
}

/// The pairs as [`parse`] reads them.
pub fn render(versions: &BTreeMap<String, String>) -> String {
    versions
        .iter()
        .map(|(group, version)| format!("{group}={version}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The version `group` is built at: `injected` first, then what `describe`
/// reports for its tags — asked only when needed — and last what the group
/// file declares.
pub fn resolve(
    group: &Group<'_>,
    injected: &BTreeMap<String, String>,
    describe: impl FnOnce(&str) -> Option<String>,
) -> String {
    if let Some(version) = injected.get(group.name) {
        return version.clone();
    }
    if let Some(described) = describe(group.prefix) {
        return described;
    }
    group.declared.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEDROCK: Group<'_> = Group {
        name: "bedrock",
        prefix: "bedrock-v",
        declared: "0.3.0",
    };

    /// Scenario _Built at a group tag_: on `main`, the in-tree crate is
    /// described by its tags, exactly at the tag or past it.
    #[test]
    fn main_reports_the_described_tag() {
        let at_tag = resolve(&BEDROCK, &BTreeMap::new(), |prefix| {
            assert_eq!(prefix, "bedrock-v");
            Some("bedrock-v0.4.0".to_owned())
        });
        assert_eq!(at_tag, "bedrock-v0.4.0");
        let past = resolve(&BEDROCK, &BTreeMap::new(), |_| {
            Some("bedrock-v0.4.0-3-gabc1234".to_owned())
        });
        assert_eq!(past, "bedrock-v0.4.0-3-gabc1234");
    }

    /// Scenario _Built on a series_: the series descends from the commit the
    /// cut tagged, so the description is the tag plus whatever was
    /// cherry-picked since — the drift is in the answer, not hidden by it.
    #[test]
    fn a_series_reports_the_described_drift() {
        let version = resolve(&BEDROCK, &BTreeMap::new(), |prefix| {
            Some(format!("{prefix}0.4.0-1-g5c50d3a"))
        });
        assert_eq!(version, "bedrock-v0.4.0-1-g5c50d3a");
    }

    /// Scenario _Source without tags_ and _A shallow build falls back to the
    /// declaration_: an injected value wins over every other source, and
    /// with nothing at all the group file's declaration is what the build
    /// claims.
    #[test]
    fn injected_values_win_and_the_declaration_is_the_floor() {
        let injected = parse("bedrock=bedrock-v0.5.0, domain=domain-v0.2.0,,broken");
        assert_eq!(injected.len(), 2);
        let version = resolve(&BEDROCK, &injected, |_| {
            panic!("an injected value is never second-guessed")
        });
        assert_eq!(version, "bedrock-v0.5.0");

        let shallow = resolve(&BEDROCK, &BTreeMap::new(), |_| None);
        assert_eq!(shallow, "0.3.0");
    }

    #[test]
    fn rendering_reads_back() {
        let mut versions = BTreeMap::new();
        versions.insert("domain".to_owned(), "domain-v0.2.0".to_owned());
        versions.insert("bedrock".to_owned(), "bedrock-v0.4.0-3-gabc1234".to_owned());
        let rendered = render(&versions);
        assert_eq!(
            rendered,
            "bedrock=bedrock-v0.4.0-3-gabc1234,domain=domain-v0.2.0"
        );
        assert_eq!(parse(&rendered), versions);
        assert!(parse("").is_empty());
    }
}
