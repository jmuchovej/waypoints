// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Where a build's library group versions come from, in order: a value
//! injected into the build, the tag the lockfile pins the group to, `git
//! describe` against the group's tags on `main`, and the version the group
//! file declares (library-minting design D6). A series builds nothing but
//! its pins, so a build there never reports a described in-tree copy; a
//! shallow clone never fails to compile, and never claims more than the file
//! says.
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

/// The tag `lockfile` pins a group to, read by the tag's shape: the first
/// entry whose source is `git+<url>?tag=<prefix>…#<rev>`. Every member of a
/// pinned group names the group's one tag, so any member's entry answers;
/// a registry crate sharing a member's name has another source and never
/// matches, and an in-tree crate has none.
pub fn pinned(lockfile: &str, prefix: &str) -> Option<String> {
    lockfile
        .lines()
        .filter_map(|line| {
            line.strip_prefix("source = \"git+")
                .and_then(|rest| rest.strip_suffix('"'))
        })
        .filter_map(|source| source.split_once("?tag=").map(|(_, rest)| rest))
        .filter_map(|rest| rest.split('#').next())
        .find(|tag| tag.starts_with(prefix) && tag.len() > prefix.len())
        .map(str::to_owned)
}

/// The version `group` is built at: `injected` first, then the tag the
/// `lockfile` pins it to, then what `describe` reports for its tags — asked
/// only when needed — and last what the group file declares.
pub fn resolve(
    group: &Group<'_>,
    injected: &BTreeMap<String, String>,
    lockfile: &str,
    describe: impl FnOnce(&str) -> Option<String>,
) -> String {
    if let Some(version) = injected.get(group.name) {
        return version.clone();
    }
    if let Some(tag) = pinned(lockfile, group.prefix) {
        return tag;
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

    const IN_TREE: &str = "\
[[package]]
name = \"bedrock\"
version = \"0.3.0\"
dependencies = [
 \"bedrock-core\",
]

[[package]]
name = \"domain\"
version = \"0.1.0\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
checksum = \"abc\"
";

    const PINNED: &str = "\
[[package]]
name = \"bedrock\"
version = \"0.4.0\"
source = \"git+https://github.com/jmuchovej/waypoints?tag=bedrock-v0.4.0#4444c8a593bf57e16e0ddca8fd05cdbbf7106c19\"
dependencies = [
 \"bedrock-core\",
]

[[package]]
name = \"bedrock-core\"
version = \"0.4.0\"
source = \"git+https://github.com/jmuchovej/waypoints?tag=bedrock-v0.4.0#4444c8a593bf57e16e0ddca8fd05cdbbf7106c19\"

[[package]]
name = \"domain\"
version = \"0.2.1\"
source = \"git+https://github.com/jmuchovej/waypoints?tag=domain-v0.2.1#4444c8a593bf57e16e0ddca8fd05cdbbf7106c19\"
";

    /// Scenario _Built at a group tag_: on `main`, the in-tree crate is
    /// described by its tags, exactly at the tag or past it.
    #[test]
    fn main_reports_the_described_tag() {
        let at_tag = resolve(&BEDROCK, &BTreeMap::new(), IN_TREE, |prefix| {
            assert_eq!(prefix, "bedrock-v");
            Some("bedrock-v0.4.0".to_owned())
        });
        assert_eq!(at_tag, "bedrock-v0.4.0");
        let past = resolve(&BEDROCK, &BTreeMap::new(), IN_TREE, |_| {
            Some("bedrock-v0.4.0-3-gabc1234".to_owned())
        });
        assert_eq!(past, "bedrock-v0.4.0-3-gabc1234");
    }

    /// Scenario _Built on a series_: the pin is the answer, exactly, and git
    /// is never asked — whatever the in-tree copy would describe as. The pin
    /// is found among several members' entries by the tag's shape, and a
    /// registry crate of a member's name is not one.
    #[test]
    fn a_series_reports_exactly_its_pin() {
        assert_eq!(
            pinned(PINNED, "bedrock-v").as_deref(),
            Some("bedrock-v0.4.0")
        );
        assert_eq!(pinned(PINNED, "domain-v").as_deref(), Some("domain-v0.2.1"));
        assert_eq!(pinned(PINNED, "outpost-v"), None);
        assert_eq!(pinned(IN_TREE, "bedrock-v"), None);
        assert_eq!(pinned(IN_TREE, "domain-v"), None);
        let version = resolve(&BEDROCK, &BTreeMap::new(), PINNED, |_| {
            panic!("a series never describes its in-tree copy")
        });
        assert_eq!(version, "bedrock-v0.4.0");
    }

    /// Scenario _Source without tags_ and _A shallow build falls back to the
    /// declaration_: an injected value wins over every other source, and
    /// with nothing at all the group file's declaration is what the build
    /// claims.
    #[test]
    fn injected_values_win_and_the_declaration_is_the_floor() {
        let injected = parse("bedrock=bedrock-v0.5.0, domain=domain-v0.2.0,,broken");
        assert_eq!(injected.len(), 2);
        let version = resolve(&BEDROCK, &injected, PINNED, |_| {
            panic!("an injected value is never second-guessed")
        });
        assert_eq!(version, "bedrock-v0.5.0");

        let shallow = resolve(&BEDROCK, &BTreeMap::new(), IN_TREE, |_| None);
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
