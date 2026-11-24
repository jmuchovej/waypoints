// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The group file: `tooling/release-groups.toml` names every workspace crate
//! outside `tooling/` as a versioned member or an unversioned internal of one
//! group, lists what each group owns beyond its crates, and declares each
//! group's version, which every member's manifest projects. The release bot
//! writes the declaration and the projections in one commit;
//! `version-integrity` holds them equal.
//!
//! The schema checks shape and is generated from these types, so the two
//! cannot drift; [`load`] checks what a schema cannot see. The app's group is
//! [`APP_GROUP`] and every other group is a library; what a group is decides
//! how it is versioned, so the file carries no key saying so.

use crate::{
    constants::{APP_GROUP, BLAME_IGNORE_FILE, GROUP_FILE, ROOT_MANIFEST, SCHEMA_FILE},
    planners::test,
    releases::version::Version,
};
use anyhow::{Context as _, Result, bail};
use cargo_metadata::Metadata;
use schemars::{JsonSchema, generate::SchemaSettings};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

/// Members here build the repository, not the product, and are never named.
const TOOLING: &str = "tooling/";

/// Which workspace crates version together, and what each group owns.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupFile {
    pub bookkeeping: Bookkeeping,
    pub group: BTreeMap<String, Group>,
}

/// What is never product work, for any group or planner.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Bookkeeping {
    /// Commits touching only these paths count for nothing.
    pub paths: Vec<String>,
    /// Commits that count for nothing, by full id: a change whose paths
    /// misattribute it, such as a break scoped to a crate that is gone,
    /// touching a group by one doc line. Kept apart from
    /// [`BLAME_IGNORE_FILE`], which the walk also reads, so `git blame`
    /// still credits the lines to it.
    #[serde(default)]
    pub ignore_revs: Vec<String>,
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
    /// The group's append-only corpus.
    pub fixtures: String,
    /// Directories the group owns beyond its crates and fixtures.
    #[serde(default)]
    pub paths: Vec<String>,
}

impl GroupFile {
    pub fn parse(text: &str) -> Result<Self> {
        let file: Self =
            toml::from_str(text).with_context(|| format!("{GROUP_FILE} does not parse"))?;
        // The walk matches a listed revision by prefix, so anything short of
        // a full id could name more than one commit.
        let full = |rev: &&String| {
            rev.len() == 40 && rev.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        };
        if let Some(short) = file.bookkeeping.ignore_revs.iter().find(|rev| !full(rev)) {
            bail!(
                "{GROUP_FILE}: `[bookkeeping].ignore-revs` lists `{short}`, which is not a full commit id"
            );
        }
        Ok(file)
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

    /// The app's group, [`APP_GROUP`], which the file has to carry.
    pub fn app_group(&self) -> Result<&str> {
        self.group
            .get_key_value(APP_GROUP)
            .map(|(name, _)| name.as_str())
            .with_context(|| format!("{GROUP_FILE} has no `[group.{APP_GROUP}]`, the app's group"))
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

    /// The group `path` counts for: the one whose crate, fixtures, or listed
    /// directory holds it. A path no group lists, `tooling/`, and bookkeeping
    /// count for none.
    pub fn owner_of(&self, path: &str) -> Option<&str> {
        if path.starts_with(TOOLING) || self.is_bookkeeping(path) {
            return None;
        }
        let owned = self.owned_paths();
        let name = test::owning_package(path, &owned)?;
        self.group
            .get_key_value(name)
            .map(|(name, _)| name.as_str())
    }

    /// Every `(group, path)` a group owns: its crates, its fixtures, and the
    /// directories it lists.
    fn owned_paths(&self) -> Vec<(String, String)> {
        self.group
            .iter()
            .flat_map(|(name, group)| {
                group
                    .members
                    .iter()
                    .chain(&group.internals)
                    .chain([&group.fixtures])
                    .chain(&group.paths)
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
        if !self.group.contains_key(APP_GROUP) {
            problems.push(format!(
                "no `[group.{APP_GROUP}]`: the app's group is named `{APP_GROUP}`"
            ));
        }

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
            for path in group.paths.iter().chain([&group.fixtures]) {
                if !exists(path) {
                    problems.push(format!("group `{name}`: `{path}` does not exist"));
                }
            }
        }

        let bookkeeping = self
            .bookkeeping
            .paths
            .iter()
            .map(String::as_str)
            .chain([BLAME_IGNORE_FILE]);
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
        bot-subjects = [ "chore(release):" ]

        [group.bedrock]
        version = "0.1.0"
        members = [ "crates/bedrock", "crates/bedrock-core" ]
        tag = "bedrock-v{version}"
        fixtures = "covenant/fixtures/bedrock/"

        [group.waypoints]
        version = "0.1.0-nightly"
        members = [ "src-tauri" ]
        internals = [ "crates/store" ]
        tag = "v{version}"
        fixtures = "covenant/fixtures/waypoints/"
        paths = [ "src/" ]
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
        assert_eq!(file.app_group().unwrap(), APP_GROUP);
        assert_eq!(file.app_version().unwrap().to_string(), "0.1.0-nightly");
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
            "tag = \"bedrock-v{version}\"",
            "tag = \"bedrock-v{version}\"\nattribution = \"header\"",
        );
        let error = GroupFile::parse(&toml).unwrap_err();
        assert!(format!("{error:#}").contains("attribution"), "{error:#}");
    }

    /// Scenario _A short id is refused_: the walk matches a listed revision
    /// by prefix, so the file names each by its full id.
    #[test]
    fn a_short_revision_is_refused() {
        let listing = |rev: &str| {
            GROUPS.replace(
                "bot-subjects",
                &format!("ignore-revs = [ \"{rev}\" ]\n        bot-subjects"),
            )
        };
        let error = GroupFile::parse(&listing("11798f9")).unwrap_err();
        assert!(format!("{error:#}").contains("`11798f9`"), "{error:#}");
        GroupFile::parse(&listing(&"1".repeat(40))).unwrap();
    }

    /// Scenario _A retired key does not parse_: what a group is decides its
    /// policy, so a key restating one is refused by name, as a misspelled
    /// key is.
    #[test]
    fn a_retired_key_does_not_parse() {
        for retired in [
            "owns = \"remainder\"",
            "counts = \"closure\"",
            "breaking = \"header\"",
            "minted = [ \"cut\" ]",
            "floors = [ \"cargo-semver-checks\" ]",
            "features-always-increment-minor = true",
            "channels = [ \"stable\" ]",
            "flavors = [ \"stable\" ]",
        ] {
            let toml = GROUPS.replace(
                "tag = \"bedrock-v{version}\"",
                &format!("tag = \"bedrock-v{{version}}\"\n{retired}"),
            );
            let error = format!("{:#}", GroupFile::parse(&toml).unwrap_err());
            let key = retired.split(' ').next().unwrap();
            assert!(error.contains(key), "{retired}: {error}");
        }
        let enforced = format!("{GROUPS}\n[enforce]\ncommits = \"conventional\"\n");
        let error = format!("{:#}", GroupFile::parse(&enforced).unwrap_err());
        assert!(error.contains("enforce"), "{error}");
    }

    /// Scenario _A file without the app's group_: the app is `waypoints` by
    /// name, and a file not carrying it describes no workspace.
    #[test]
    fn a_file_without_the_app_s_group_is_refused() {
        let toml = GROUPS.replace("[group.waypoints]", "[group.app]");
        let found = problems(&toml, &["crates/bedrock", "crates/bedrock-core"]);
        assert_refused(&found, &["[group.waypoints]", "the app's group"]);
        assert!(GroupFile::parse(&toml).unwrap().app_group().is_err());
    }

    /// Scenarios _Listed path that does not exist_, _Listed path under
    /// tooling_, and _Overlapping listed path_: a listed directory is held to
    /// what a fixtures directory is held to.
    #[test]
    fn a_listed_path_is_held_like_a_fixture() {
        let file = GroupFile::parse(GROUPS).unwrap();
        let found = file.problems(
            &directories(&tree(&[])),
            |path| path != "src/",
            &BTreeSet::new(),
        );
        assert_refused(&found, &["waypoints", "src/", "does not exist"]);

        let under_tooling =
            GROUPS.replace(r#"paths = [ "src/" ]"#, r#"paths = [ "tooling/bin/" ]"#);
        assert_refused(
            &problems(&under_tooling, &tree(&[])),
            &["tooling/bin", "waypoints", "tooling/"],
        );

        let nested = GROUPS.replace(
            r#"paths = [ "src/" ]"#,
            r#"paths = [ "crates/bedrock/src/" ]"#,
        );
        assert_refused(
            &problems(&nested, &tree(&[])),
            &["crates/bedrock", "crates/bedrock/src"],
        );
    }

    /// Fixtures and listed directories belong to their group, a nested path
    /// to the longer prefix, and a path no group lists to none.
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
        assert_eq!(touched(&["src/app/a.vue"]), ["waypoints"]);
        assert_eq!(touched(&["covenant/fixtures/bedrock/a.json"]), ["bedrock"]);
        assert_eq!(
            touched(&["crates/bedrock-core/src/lib.rs", "src-tauri/src/lib.rs"]),
            ["bedrock", "waypoints"]
        );
        assert!(
            touched(&[
                "docs/CONTRIBUTING.md",
                "website/content/index.md",
                "tooling/xtask/src/lib.rs"
            ])
            .is_empty()
        );
        assert_eq!(file.owner_of("src/app/a.vue"), Some("waypoints"));
        assert_eq!(file.owner_of("docs/CONTRIBUTING.md"), None);
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
