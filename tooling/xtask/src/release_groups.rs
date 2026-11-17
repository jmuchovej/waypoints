// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The group file: `tooling/release-groups.toml` names every workspace crate
//! outside `tooling/` as a versioned member or an unversioned internal of one
//! group, and declares each group's version, which every member's manifest
//! projects. The release bot writes the declaration and the projections in
//! one commit; `version-integrity` holds them equal.
//!
//! The schema checks shape and is generated from these types, so the two
//! cannot drift; [`load`] checks what a schema cannot see. Keys whose meaning
//! belongs to a later change are parsed and shape-checked here, and nothing
//! acts on them yet.

use crate::{
    Channel, Flavor,
    planners::test,
    releases::version::{Transition, Version},
};
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

/// The only author who writes a version or a bookkeeping subject, and whose
/// every commit is bookkeeping to the planners.
pub const RELEASE_BOT: &str = "wp-botswain[bot]";

/// Where every group's tag is minted, and what a series pins a library from.
pub const REPOSITORY: &str = "https://github.com/jmuchovej/waypoints";

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
    /// The group's version, which every member's `[package].version`
    /// projects: bare for a library, with its channel suffix for the app.
    pub version: String,
    /// Crates carrying the group's version.
    pub members: Vec<String>,
    /// Crates the group owns and never versions.
    #[serde(default)]
    pub internals: Vec<String>,
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

    /// The version `group` declares, which its members project.
    pub fn version_of(&self, group: &str) -> Result<Version> {
        let named = self
            .group
            .get(group)
            .with_context(|| format!("no group named `{group}`"))?;
        Version::parse(&named.version).with_context(|| {
            format!(
                "group `{group}`: version `{}` is not a semantic version",
                named.version
            )
        })
    }

    /// The app's group: the one owning the remainder.
    pub fn app_group(&self) -> Result<&str> {
        self.remainder_group()
            .context("no group owns the remainder; the app's group names `owns = \"remainder\"`")
    }

    /// The version the tree ships as — the app's, channel suffix included.
    pub fn app_version(&self) -> Result<Version> {
        self.version_of(self.app_group()?)
    }

    /// Every member declaring anything but the exact version its group
    /// declares here, prerelease included — `versions` being each workspace
    /// crate's declared version by its directory. The lockstep rule:
    /// `version-integrity` holds a pull request to it, and [`load`] a
    /// planner's tree. A pinned group's members are out of the workspace and
    /// declare nothing here.
    pub fn lockstep_problems(&self, versions: &BTreeMap<String, String>) -> Vec<String> {
        let mut found = Vec::new();
        for (name, group) in &self.group {
            for member in &group.members {
                if let Some(declared) = versions.get(member).filter(|v| **v != group.version) {
                    found.push(format!(
                        "`{member}/Cargo.toml` declares {declared}, but its group `{name}` declares {expected} in `{GROUP_FILE}`",
                        expected = group.version,
                    ));
                }
            }
        }
        found
    }

    /// The groups owning any of `paths`.
    pub fn touched<'a>(&self, paths: impl Iterator<Item = &'a str>) -> BTreeSet<String> {
        let owned = self.owned_paths();
        paths
            .filter_map(|path| test::owning_package(path, &owned))
            .map(str::to_owned)
            .collect()
    }

    /// Whether `path` is bookkeeping, which counts for no group.
    pub fn is_bookkeeping(&self, path: &str) -> bool {
        self.bookkeeping
            .paths
            .iter()
            .any(|dir| path.starts_with(dir.as_str()))
    }

    /// The group `path` counts for: the one whose crate or fixtures hold it,
    /// else the group owning the remainder. `tooling/` and bookkeeping count
    /// for none.
    pub fn owner_of(&self, path: &str) -> Option<&str> {
        if path.starts_with(TOOLING) || self.is_bookkeeping(path) {
            return None;
        }
        match test::owning_package(path, &self.owned_paths()) {
            Some(name) => self
                .group
                .get_key_value(name)
                .map(|(name, _)| name.as_str()),
            None => self.remainder_group(),
        }
    }

    /// The group owning every path no other group does.
    pub fn remainder_group(&self) -> Option<&str> {
        self.group
            .iter()
            .find(|(_, group)| matches!(group.owns, Some(Owns::Remainder)))
            .map(|(name, _)| name.as_str())
    }

    /// The groups counting every group's paths as their own work.
    pub fn closure_groups(&self) -> impl Iterator<Item = &str> {
        self.group
            .iter()
            .filter(|(_, group)| matches!(group.counts, Some(Counts::Closure)))
            .map(|(name, _)| name.as_str())
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
    /// members sit at `directories` and whose root manifest pins the
    /// `pinned` groups — a series' libraries, on disk and out of the
    /// workspace, described by their pins rather than by membership.
    fn problems(
        &self,
        directories: &[(String, String)],
        exists: impl Fn(&str) -> bool,
        pinned: &BTreeSet<String>,
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
            if Version::parse(&group.version).is_err() {
                problems.push(format!(
                    "group `{name}`: version `{}` is not a semantic version",
                    group.version
                ));
            }
            for path in group.members.iter().chain(&group.internals) {
                if !pinned.contains(name) && !directories.iter().any(|(_, dir)| dir == path) {
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
/// that does not describe it or that its members do not project — the
/// lockstep rule, so no planner plans from a projection `version-integrity`
/// would refuse.
pub fn load(metadata: &Metadata) -> Result<GroupFile> {
    let file = load_unprojected(metadata)?;
    let problems = file.lockstep_problems(&declared_versions(metadata));
    if !problems.is_empty() {
        bail!(
            "{GROUP_FILE} is not projected into this workspace:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(file)
}

/// Every workspace crate's declared version, by its directory.
pub fn declared_versions(metadata: &Metadata) -> BTreeMap<String, String> {
    let root = metadata.workspace_root.as_std_path();
    metadata
        .workspace_packages()
        .iter()
        .filter_map(|package| {
            let dir = package.manifest_path.as_std_path().parent()?;
            let relative = dir.strip_prefix(root).ok()?;
            Some((
                relative.to_string_lossy().into_owned(),
                package.version.to_string(),
            ))
        })
        .collect()
}

/// [`load`] without the lockstep rule: the file as it stands, for the
/// projection that makes the members agree with it and for the check that
/// reports a disagreement rather than refusing on it. A series' root
/// manifest pins its library groups, whose crates are on disk and out of the
/// workspace; those are described by their pins.
pub fn load_unprojected(metadata: &Metadata) -> Result<GroupFile> {
    let root = metadata.workspace_root.as_std_path();
    let path = root.join(GROUP_FILE);
    let text =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let file = GroupFile::parse(&text)?;

    let manifest = fs::read_to_string(root.join(ROOT_MANIFEST))
        .with_context(|| format!("Failed to read {ROOT_MANIFEST}"))?;
    let pinned = pins_in(ROOT_MANIFEST, &manifest, &file)?
        .into_iter()
        .map(|pin| pin.group)
        .collect();
    let problems = file.problems(
        &test::package_directories(metadata),
        |path| root.join(path).exists(),
        &pinned,
    );
    if !problems.is_empty() {
        bail!(
            "{GROUP_FILE} does not describe this workspace:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(file)
}

/// The workspace's root manifest: its members, and the one place a library
/// dependency is declared — where a series pins its library groups.
pub const ROOT_MANIFEST: &str = "Cargo.toml";

/// A library group a manifest pins: a `git` dependency naming the group's
/// tag. Read by the tag's shape against each group's template, so no map
/// from crate names to groups is needed — on a series the library crates are
/// no longer workspace members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    pub group: String,
    pub tag: String,
    /// The manifest declaring it, relative to the workspace root.
    pub manifest: String,
}

/// The tables a manifest declares dependencies in.
const DEPENDENCY_TABLES: [&[&str]; 4] = [
    &["workspace", "dependencies"],
    &["dependencies"],
    &["dev-dependencies"],
    &["build-dependencies"],
];

fn nested<'a>(table: &'a toml::Table, keys: &[&str]) -> Option<&'a toml::Table> {
    let mut current = table;
    for key in keys {
        current = current.get(*key)?.as_table()?;
    }
    Some(current)
}

/// The part of a group's tag `template` before its version.
pub fn tag_prefix(template: &str) -> Result<&str> {
    template
        .split_once("{version}")
        .map(|(prefix, _)| prefix)
        .filter(|prefix| !prefix.is_empty())
        .with_context(|| format!("tag template `{template}` has no prefix before `{{version}}`"))
}

/// The group whose tag template `tag` fits.
pub fn group_of_tag<'a>(tag: &str, groups: &'a GroupFile) -> Option<&'a str> {
    groups
        .group
        .iter()
        .find(|(_, group)| {
            group
                .tag
                .split_once("{version}")
                .is_some_and(|(prefix, _)| !prefix.is_empty() && tag.starts_with(prefix))
        })
        .map(|(name, _)| name.as_str())
}

/// Every pin the manifest at `path` declares.
pub fn pins_in(path: &str, manifest: &str, groups: &GroupFile) -> Result<Vec<Pin>> {
    let table: toml::Table = manifest
        .parse()
        .with_context(|| format!("{path} is not TOML"))?;
    let mut pins = Vec::new();
    for keys in DEPENDENCY_TABLES {
        let Some(dependencies) = nested(&table, keys) else {
            continue;
        };
        for dependency in dependencies.values() {
            let Some(spec) = dependency.as_table() else {
                continue;
            };
            let git = spec.get("git").and_then(toml::Value::as_str);
            let tag = spec.get("tag").and_then(toml::Value::as_str);
            let (Some(_), Some(tag)) = (git, tag) else {
                continue;
            };
            if let Some(group) = group_of_tag(tag, groups) {
                pins.push(Pin {
                    group: group.to_owned(),
                    tag: tag.to_owned(),
                    manifest: path.to_owned(),
                });
            }
        }
    }
    Ok(pins)
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
        version = "0.1.0"
        members = [ "crates/bedrock", "crates/bedrock-core" ]
        tag = "bedrock-v{version}"
        breaking = "header"
        floors = [ "cargo-semver-checks" ]
        fixtures = "covenant/fixtures/bedrock/"
        minted = [ "cut" ]

        [group.waypoints]
        version = "0.1.0-nightly"
        members = [ "src-tauri" ]
        internals = [ "crates/store" ]
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
    /// named path, with no group pinned.
    fn problems(toml: &str, members: &[&str]) -> Vec<String> {
        GroupFile::parse(toml)
            .unwrap()
            .problems(&directories(members), |_| true, &BTreeSet::new())
    }

    /// A series: a pinned group's crates are on disk and out of the
    /// workspace, and the file still describes it; unpinned, their absence
    /// is refused as on `main`.
    #[test]
    fn a_pinned_group_s_crates_may_leave_the_workspace() {
        let file = GroupFile::parse(GROUPS).unwrap();
        let without_bedrock = directories(&["crates/store", "src-tauri", "tooling/xtask"]);
        let pinned: BTreeSet<String> = ["bedrock".to_owned()].into();
        assert_eq!(
            file.problems(&without_bedrock, |_| true, &pinned),
            Vec::<String>::new()
        );
        assert_refused(
            &file.problems(&without_bedrock, |_| true, &BTreeSet::new()),
            &["bedrock", "crates/bedrock"],
        );

        let manifest = indoc::indoc! {r#"
            [workspace]
            members = [ "crates/store", "src-tauri" ]

            [workspace.dependencies]
            bedrock = { git = "https://github.com/jmuchovej/waypoints", tag = "bedrock-v0.2.0" }
            bedrock-core = { git = "https://github.com/jmuchovej/waypoints", tag = "bedrock-v0.2.0" }
            store = { path = "crates/store" }
        "#};
        let read: Vec<(String, String)> = pins_in("Cargo.toml", manifest, &file)
            .unwrap()
            .into_iter()
            .map(|pin| (pin.group, pin.tag))
            .collect();
        assert_eq!(
            read,
            [
                ("bedrock".to_owned(), "bedrock-v0.2.0".to_owned()),
                ("bedrock".to_owned(), "bedrock-v0.2.0".to_owned()),
            ]
        );
        assert_eq!(group_of_tag("outpost-v0.2.0", &file), None);
        assert_eq!(group_of_tag("bedrock-v0.2.0", &file), Some("bedrock"));
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

    /// Every group declares its version in the file, and it is read as
    /// semver — the app's with its channel suffix, a library's bare.
    #[test]
    fn a_version_that_is_not_semver_is_refused() {
        let toml = GROUPS.replace(r#"version = "0.1.0""#, r#"version = "0.1""#);
        assert_refused(&problems(&toml, &tree(&[])), &["bedrock", "0.1"]);

        // A group declaring no version — and the anchor that used to stand in
        // for one — does not parse.
        let anchored = GROUPS.replace(
            r#"tag = "bedrock-v{version}""#,
            "anchor = \"crates/bedrock\"\n        tag = \"bedrock-v{version}\"",
        );
        let refused = format!("{:#}", GroupFile::parse(&anchored).unwrap_err());
        assert!(refused.contains("anchor"), "{refused}");
        let unversioned = GROUPS.replace(r#"version = "0.1.0""#, "");
        let refused = format!("{:#}", GroupFile::parse(&unversioned).unwrap_err());
        assert!(refused.contains("version"), "{refused}");

        let file = GroupFile::parse(GROUPS).unwrap();
        assert_eq!(file.version_of("bedrock").unwrap(), Version::new(0, 1, 0));
        assert!(file.version_of("outpost").is_err());
        assert!(file.app_group().is_err());

        let with_app = GroupFile::parse(&GROUPS.replace(
            r#"tag = "v{version}""#,
            "tag = \"v{version}\"\n        owns = \"remainder\"",
        ))
        .unwrap();
        assert_eq!(with_app.app_group().unwrap(), "waypoints");
        assert_eq!(with_app.app_version().unwrap().to_string(), "0.1.0-nightly");
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
            let found = file.problems(
                &directories(&tree(&[])),
                |path| path != missing,
                &BTreeSet::new(),
            );
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
