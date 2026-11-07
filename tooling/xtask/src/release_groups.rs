// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The group file: `tooling/release-groups.toml` names every workspace crate
//! outside `tooling/` as a versioned member or an unversioned internal of one
//! group. Versions stay in the manifests, so the file takes no bookkeeping
//! commits.
//!
//! The schema checks shape and is generated from these types, so the two
//! cannot drift; [`load`] checks what a schema cannot see. Keys whose meaning
//! belongs to a later change are parsed and shape-checked here, and nothing
//! acts on them yet.

use crate::{Channel, Flavor, planners::test, releases::version::Transition};
use anyhow::{Context as _, Result, bail};
use cargo_metadata::Metadata;
use schemars::{JsonSchema, generate::SchemaSettings};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

/// The group file, relative to the workspace root.
pub const GROUP_FILE: &str = "tooling/release-groups.toml";

/// Its generated schema, relative to the workspace root.
pub const SCHEMA_FILE: &str = "tooling/schemas/release-groups.schema.json";

/// Members here build the repository, not the product, and are never named.
const TOOLING: &str = "tooling/";

/// Which workspace crates version together, and what a pull request may claim
/// about them.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupFile {
    pub bookkeeping: Bookkeeping,
    pub group: BTreeMap<String, Group>,
    pub enforce: Enforce,
}

/// What is never product work, for any group or planner.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Bookkeeping {
    /// Commits touching only these paths count for nothing.
    pub paths: Vec<String>,
    /// The file listing treewide formatting passes.
    pub ignore_revs: String,
    /// Subject prefixes reserved to the release bot.
    pub bot_subjects: Vec<String>,
}

/// One versioned line of crates.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Group {
    /// Crates carrying the group's version.
    pub members: Vec<String>,
    /// Crates the group owns and never versions.
    #[serde(default)]
    pub internals: Vec<String>,
    /// The member whose `[package].version` is the group's version.
    pub anchor: String,
    /// The group's tag, with `{version}` standing in for its version.
    pub tag: String,
    /// What marks a breaking change for this group.
    pub breaking: Breaking,
    /// What can force a cut above what the commits declare.
    pub floors: Vec<Floor>,
    /// The group's append-only corpus.
    pub fixtures: String,
    /// The transitions that mint this group's versions.
    pub minted: Vec<Transition>,
    /// Paths the group owns beyond its crates and fixtures.
    #[serde(default)]
    pub owns: Option<Owns>,
    /// Which paths count as work for this group.
    #[serde(default)]
    pub counts: Option<Counts>,
    /// Whether a `feat` always moves the minor, even before `v1`.
    #[serde(default)]
    pub features_always_increment_minor: bool,
    /// The channels the group ships through.
    #[serde(default)]
    pub channels: Vec<Channel>,
    /// The flavors every crate in the group declares.
    #[serde(default)]
    pub flavors: Vec<Flavor>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub enum Breaking {
    /// `!` in the group's own header.
    #[serde(rename = "header")]
    Header,
    /// The `Covenant: break` trailer.
    #[serde(rename = "trailer:Covenant: break")]
    CovenantTrailer,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Floor {
    CargoSemverChecks,
    CovenantFixtures,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Owns {
    /// Everything no other group owns, minus `tooling/` and bookkeeping.
    Remainder,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Counts {
    /// Its own paths plus every group's: the binary ships them.
    Closure,
}

/// The pull-request checks `version-integrity` carries.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Enforce {
    pub commits: Commits,
    pub summary_type: SummaryType,
    pub version_writes: VersionWrites,
    pub fixtures: Fixtures,
    pub schema: SchemaRule,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Commits {
    /// A landing message is a conventional commit.
    Conventional,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SummaryType {
    /// A multi-group change lands as `summary`, one body header per group.
    Summary,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum VersionWrites {
    /// Only `wp-botswain[bot]` changes a member's `[package].version`.
    Botswain,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Fixtures {
    /// A fixture changes only under the `Covenant: break` trailer.
    AppendOnly,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaRule {
    /// The committed schema equals the regenerated one.
    Current,
}

impl GroupFile {
    pub fn parse(text: &str) -> Result<Self> {
        toml::from_str(text).with_context(|| format!("{GROUP_FILE} does not parse"))
    }

    /// The crates carrying a version, across every group.
    pub fn versioned_members(&self) -> impl Iterator<Item = &str> {
        self.group
            .values()
            .flat_map(|group| &group.members)
            .map(String::as_str)
    }

    /// The members of the group anchored at `anchor`, the crates one version
    /// write has to set together.
    pub fn members_anchored_at(&self, anchor: &str) -> Option<&[String]> {
        self.group
            .values()
            .find(|group| group.anchor == anchor)
            .map(|group| group.members.as_slice())
    }

    /// The groups owning any of `paths`.
    pub fn touched<'a>(&self, paths: impl Iterator<Item = &'a str>) -> BTreeSet<String> {
        let owned = self.owned_paths();
        paths
            .filter_map(|path| test::owning_package(path, &owned))
            .map(str::to_owned)
            .collect()
    }

    /// Every `(group, path)` a group owns: its crates and its fixtures.
    fn owned_paths(&self) -> Vec<(String, String)> {
        self.group
            .iter()
            .flat_map(|(name, group)| {
                group
                    .members
                    .iter()
                    .chain(&group.internals)
                    .chain([&group.fixtures])
                    .map(|path| (name.clone(), path.trim_end_matches('/').to_owned()))
            })
            .collect()
    }

    /// What is wrong with this file as a description of a workspace whose
    /// members sit at `directories`.
    fn problems(
        &self,
        directories: &[(String, String)],
        exists: impl Fn(&str) -> bool,
    ) -> Vec<String> {
        let mut problems = Vec::new();

        let mut named: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (group, path) in self.owned_paths() {
            named.entry(path).or_default().push(group);
        }

        for (path, groups) in &named {
            let by = groups.join("` and `");
            if path.starts_with(TOOLING) {
                problems.push(format!(
                    "`{path}` is named by `{by}`, but tooling/ builds the repository and must not be named"
                ));
            }
            if groups.len() > 1 {
                problems.push(format!("`{path}` is named more than once: by `{by}`"));
            }
        }

        for path in named.keys() {
            let prefix = format!("{path}/");
            for nested in named.keys().filter(|other| other.starts_with(&prefix)) {
                problems.push(format!("owned paths overlap: `{path}` holds `{nested}`"));
            }
        }

        for (name, group) in &self.group {
            if !group.members.contains(&group.anchor) {
                problems.push(format!(
                    "group `{name}`: anchor `{}` is not among its members",
                    group.anchor
                ));
            }
            for path in group.members.iter().chain(&group.internals) {
                if !directories.iter().any(|(_, dir)| dir == path) {
                    problems.push(format!(
                        "group `{name}`: `{path}` is not a workspace member"
                    ));
                }
            }
            if !exists(&group.fixtures) {
                problems.push(format!(
                    "group `{name}`: `{}` does not exist",
                    group.fixtures
                ));
            }
        }

        let bookkeeping = self
            .bookkeeping
            .paths
            .iter()
            .chain([&self.bookkeeping.ignore_revs]);
        for path in bookkeeping.filter(|path| !exists(path)) {
            problems.push(format!("bookkeeping: `{path}` does not exist"));
        }

        for (_, dir) in directories {
            if !dir.starts_with(TOOLING) && !named.contains_key(dir) {
                problems.push(format!("workspace member `{dir}` is not named"));
            }
        }

        problems
    }
}

/// Reads the group file of the workspace `metadata` describes, refusing one
/// that does not describe it.
pub fn load(metadata: &Metadata) -> Result<GroupFile> {
    let root = metadata.workspace_root.as_std_path();
    let path = root.join(GROUP_FILE);
    let text =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let file = GroupFile::parse(&text)?;

    let problems = file.problems(&test::package_directories(metadata), |path| {
        root.join(path).exists()
    });
    if !problems.is_empty() {
        bail!(
            "{GROUP_FILE} does not describe this workspace:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(file)
}

/// The schema's bytes, as they are committed at [`SCHEMA_FILE`].
pub fn schema() -> Result<String> {
    let schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<GroupFile>();
    Ok(serde_json::to_string_pretty(&schema).context("Failed to serialize the schema")? + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const GROUPS: &str = r#"
        [bookkeeping]
        paths = [ "nix/releases/" ]
        ignore-revs = ".git-blame-ignore-revs"
        bot-subjects = [ "chore(release):" ]

        [group.bedrock]
        members = [ "crates/bedrock", "crates/bedrock-core" ]
        anchor = "crates/bedrock"
        tag = "bedrock-v{version}"
        breaking = "header"
        floors = [ "cargo-semver-checks" ]
        fixtures = "covenant/fixtures/bedrock/"
        minted = [ "cut" ]

        [group.waypoints]
        members = [ "src-tauri" ]
        internals = [ "crates/store" ]
        anchor = "src-tauri"
        tag = "v{version}"
        breaking = "trailer:Covenant: break"
        floors = [ "covenant-fixtures" ]
        fixtures = "covenant/fixtures/waypoints/"
        minted = [ "cut", "advance", "promote", "patch" ]

        [enforce]
        commits = "conventional"
        summary-type = "summary"
        version-writes = "botswain"
        fixtures = "append-only"
        schema = "current"
    "#;

    fn directories(paths: &[&str]) -> Vec<(String, String)> {
        paths
            .iter()
            .map(|path| {
                let name = path.rsplit('/').next().unwrap();
                (name.to_owned(), (*path).to_owned())
            })
            .collect()
    }

    /// What's wrong with `toml` over a tree holding `members` and every other
    /// named path.
    fn problems(toml: &str, members: &[&str]) -> Vec<String> {
        GroupFile::parse(toml)
            .unwrap()
            .problems(&directories(members), |_| true)
    }

    fn tree(extra: &[&'static str]) -> Vec<&'static str> {
        let mut members = vec![
            "crates/bedrock",
            "crates/bedrock-core",
            "crates/store",
            "src-tauri",
            "tooling/gh-workflow",
            "tooling/xtask",
        ];
        members.extend(extra);
        members
    }

    fn assert_refused(problems: &[String], names: &[&str]) {
        assert!(
            problems
                .iter()
                .any(|problem| names.iter().all(|name| problem.contains(name))),
            "no problem names {names:?}: {problems:?}"
        );
    }

    /// `tooling/xtask` and `tooling/gh-workflow` are members, and unnamed.
    #[test]
    fn a_complete_file_has_no_problems() {
        assert_eq!(problems(GROUPS, &tree(&[])), Vec::<String>::new());
    }

    #[test]
    fn a_new_member_is_refused_until_it_is_named() {
        let found = problems(GROUPS, &tree(&["crates/importer"]));
        assert_refused(&found, &["crates/importer"]);
    }

    #[test]
    fn a_crate_named_by_two_groups_names_both() {
        let toml = GROUPS.replace(
            r#"internals = [ "crates/store" ]"#,
            r#"internals = [ "crates/store", "crates/bedrock-core" ]"#,
        );
        assert_refused(
            &problems(&toml, &tree(&[])),
            &["crates/bedrock-core", "bedrock", "waypoints"],
        );
    }

    #[test]
    fn a_tooling_member_is_refused_when_named() {
        for member in ["tooling/xtask", "tooling/gh-workflow"] {
            let toml = GROUPS.replace(
                r#"internals = [ "crates/store" ]"#,
                &format!(r#"internals = [ "crates/store", "{member}" ]"#),
            );
            assert_refused(&problems(&toml, &tree(&[])), &[member]);
        }
    }

    #[test]
    fn an_anchor_outside_its_members_is_refused() {
        let toml = GROUPS.replace(r#"anchor = "crates/bedrock""#, r#"anchor = "crates/store""#);
        assert_refused(&problems(&toml, &tree(&[])), &["bedrock", "crates/store"]);
    }

    #[test]
    fn a_name_that_is_no_member_is_refused() {
        let found = problems(GROUPS, &["crates/bedrock", "crates/store", "src-tauri"]);
        assert_refused(&found, &["bedrock", "crates/bedrock-core"]);
    }

    #[test]
    fn a_missing_path_is_refused() {
        let file = GroupFile::parse(GROUPS).unwrap();
        for missing in [
            "covenant/fixtures/waypoints/",
            "nix/releases/",
            ".git-blame-ignore-revs",
        ] {
            let found = file.problems(&directories(&tree(&[])), |path| path != missing);
            assert_refused(&found, &[missing]);
        }
    }

    #[test]
    fn overlapping_owned_paths_are_refused() {
        let toml = GROUPS.replace(
            r#"fixtures = "covenant/fixtures/waypoints/""#,
            r#"fixtures = "crates/bedrock/fixtures""#,
        );
        assert_refused(
            &problems(&toml, &tree(&[])),
            &["crates/bedrock", "crates/bedrock/fixtures"],
        );
    }

    #[test]
    fn a_misspelled_key_does_not_parse() {
        let toml = GROUPS.replace(
            "breaking = \"header\"",
            "breaking = \"header\"\nattribution = \"header\"",
        );
        let error = GroupFile::parse(&toml).unwrap_err();
        assert!(format!("{error:#}").contains("attribution"), "{error:#}");
    }

    /// Fixtures belong to their group, and a nested path to the longer prefix.
    #[test]
    fn a_path_resolves_to_the_group_owning_it() {
        let file = GroupFile::parse(GROUPS).unwrap();
        let touched = |paths: &[&str]| {
            file.touched(paths.iter().copied())
                .into_iter()
                .collect::<Vec<_>>()
        };
        assert_eq!(touched(&["crates/bedrock/src/lib.rs"]), ["bedrock"]);
        assert_eq!(touched(&["crates/store/src/db.rs"]), ["waypoints"]);
        assert_eq!(touched(&["covenant/fixtures/bedrock/a.json"]), ["bedrock"]);
        assert_eq!(
            touched(&["crates/bedrock-core/src/lib.rs", "src-tauri/src/lib.rs"]),
            ["bedrock", "waypoints"]
        );
        assert!(touched(&["docs/CONTRIBUTING.md", "tooling/xtask/src/lib.rs"]).is_empty());
    }

    /// The internal is not versioned, so a version write to it is no claim.
    #[test]
    fn only_members_carry_a_version() {
        let file = GroupFile::parse(GROUPS).unwrap();
        let versioned: Vec<&str> = file.versioned_members().collect();
        assert_eq!(
            versioned,
            ["crates/bedrock", "crates/bedrock-core", "src-tauri"]
        );
    }

    #[test]
    fn the_group_file_describes_this_workspace() {
        let file = load(&crate::workspace::load().unwrap()).unwrap();
        let groups: Vec<&str> = file.group.keys().map(String::as_str).collect();
        assert_eq!(
            groups,
            ["bedrock", "domain", "outpost", "transload", "waypoints"]
        );
    }

    /// The parser and what tombi lints against cannot drift.
    #[test]
    fn the_committed_schema_is_the_generated_one() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let committed = fs::read_to_string(root.join(SCHEMA_FILE)).unwrap();
        assert_eq!(committed, schema().unwrap());
    }
}
