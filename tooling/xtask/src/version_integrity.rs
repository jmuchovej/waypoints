// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The pull-request check in front of every version claim: who wrote a
//! version, whether a group's members agree with its anchor, whether the
//! landing message is conventional and, for a multi-group change, complete, and
//! whether a frozen fixture was rewritten undeclared.
//!
//! It reads the event payload and the diff from the merge base, so it needs no
//! token. The title and description are what the merge commit carries; branch
//! commits underneath are not checked.

use crate::{
    release_groups::{self, GroupFile, SCHEMA_FILE},
    tools::git,
};
use anyhow::{Context as _, Result};
use cargo_metadata::Metadata;
use git_conventional::Commit;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

/// The only author who writes a version or a bookkeeping subject.
const RELEASE_BOT: &str = "wp-botswain[bot]";

/// Frozen inputs: append-only unless the merge message declares otherwise.
const FIXTURES: &str = "covenant/fixtures/";

/// The multi-group subject type. A summary's subject has no scope and no `!`,
/// since those belong to each group's header and `.config/cliff.toml` skips
/// exactly `summary:`.
const SUMMARY: &str = "summary";

/// The types `docs/CONTRIBUTING.md` lists, in its order.
const TYPES: [&str; 11] = [
    "feat", "fix", "perf", "refactor", "docs", "test", "style", "build", "ci", "chore", SUMMARY,
];

/// A pull request, as the `pull_request` event carries it.
#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub title: String,
    /// Null when the description was left empty.
    pub body: Option<String>,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
}

impl PullRequest {
    pub fn from_event(payload: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct Event {
            pull_request: PullRequest,
        }
        let event: Event =
            serde_json::from_str(payload).context("The event payload carries no pull request")?;
        Ok(event.pull_request)
    }

    /// GitHub's web form submits CRLF, which the parsers below would read as
    /// part of each line.
    fn description(&self) -> String {
        self.body
            .as_deref()
            .unwrap_or_default()
            .replace("\r\n", "\n")
            .trim()
            .to_owned()
    }

    /// The message the merge commit will carry.
    fn message(&self) -> String {
        format!("{}\n\n{}", self.title.trim(), self.description())
            .trim_end()
            .to_owned()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Added,
    Modified,
    Deleted,
}

/// One path the pull request changes. A rename is a deletion and an addition,
/// so it touches both paths' groups and reads as a rewrite of the old fixture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub status: Status,
    pub path: String,
}

/// NUL-separated, so a non-ASCII path arrives as itself rather than quoted, and
/// still resolves to its group.
fn diff_args(base: &str) -> [&str; 6] {
    ["diff", "--name-status", "--no-renames", "-z", base, "HEAD"]
}

fn parse_changes(output: &str) -> Vec<Change> {
    let mut fields = output.split('\0');
    let mut changes = Vec::new();
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        let status = match status {
            "A" => Status::Added,
            "D" => Status::Deleted,
            _ => Status::Modified,
        };
        changes.push(Change {
            status,
            path: path.to_owned(),
        });
    }
    changes
}

/// Every path changed since `base`.
pub fn changes(base: &str) -> Result<Vec<Change>> {
    Ok(parse_changes(&git::run(&diff_args(base))?))
}

/// The versioned members' manifests whose `[package].version` differs between
/// `sides(path)`'s two texts. A manifest that appears or disappears is a crate
/// joining or leaving, not a version write.
fn differing_versions(
    groups: &GroupFile,
    changes: &[Change],
    sides: impl Fn(&str) -> Result<(String, String)>,
) -> Result<Vec<String>> {
    let manifests: BTreeSet<String> = groups
        .versioned_members()
        .map(|member| format!("{member}/Cargo.toml"))
        .collect();

    let mut writes = Vec::new();
    for change in changes {
        if change.status != Status::Modified || !manifests.contains(&change.path) {
            continue;
        }
        let (before, after) = sides(&change.path)?;
        if package_version(&before)? != package_version(&after)? {
            writes.push(change.path.clone());
        }
    }
    Ok(writes)
}

fn package_version(manifest: &str) -> Result<Option<toml::Value>> {
    let table: toml::Table = manifest
        .parse()
        .context("A member's manifest is not TOML")?;
    Ok(table
        .get("package")
        .and_then(|package| package.get("version"))
        .cloned())
}

/// Every versioned member's manifest whose `[package].version` moved since
/// `base`.
pub fn version_writes(groups: &GroupFile, changes: &[Change], base: &str) -> Result<Vec<String>> {
    let show = |revision: &str, path: &str| git::run(&["show", &format!("{revision}:{path}")]);
    differing_versions(groups, changes, |path| {
        Ok((show(base, path)?, show("HEAD", path)?))
    })
}

/// Every workspace crate's declared version, by its directory.
fn declared_versions(metadata: &Metadata) -> BTreeMap<String, String> {
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

/// Every member declaring anything but its anchor's exact version, prerelease
/// included.
fn lockstep_violations(groups: &GroupFile, versions: &BTreeMap<String, String>) -> Vec<String> {
    let mut found = Vec::new();
    for group in groups.group.values() {
        let Some(expected) = versions.get(&group.anchor) else {
            continue;
        };
        for member in &group.members {
            if let Some(declared) = versions.get(member).filter(|v| *v != expected) {
                found.push(format!(
                    "`{member}/Cargo.toml` declares {declared}, but its group's anchor `{anchor}/Cargo.toml` declares {expected}",
                    anchor = group.anchor,
                ));
            }
        }
    }
    found
}

/// Whether `path` is frozen. A group's own README describes its directory, so
/// it is not; a `README.md` any deeper is data, since import corpora are
/// Markdown.
fn is_fixture(path: &str) -> bool {
    path.strip_prefix(FIXTURES).is_some_and(|rest| {
        !rest
            .split_once('/')
            .is_some_and(|(_group, file)| file == "README.md")
    })
}

/// Whether the message carries the `Covenant: break` trailer.
fn declares_covenant_break(commit: &Commit<'_>) -> bool {
    commit.footers().iter().any(|footer| {
        footer.token().as_str() == "Covenant" && footer.value().lines().next() == Some("break")
    })
}

/// What's wrong with a summary's subject and headers, against what it touched.
fn summary_violations(
    title: &str,
    description: &str,
    touched: &BTreeSet<String>,
    groups: &GroupFile,
) -> Vec<String> {
    let mut found = Vec::new();
    if !title.starts_with("summary:") {
        found.push(format!(
            "the subject `{title}` must read `summary: …`; scopes and `!` belong to each group's header"
        ));
    }

    let mut headers: BTreeMap<&str, usize> = BTreeMap::new();
    for line in description.lines() {
        let Ok(header) = Commit::parse(line) else {
            continue;
        };
        let kind = header.type_();
        if kind.as_str() == SUMMARY || !TYPES.contains(&kind.as_str()) {
            continue;
        }
        let group = header
            .scope()
            .map(|scope| scope.as_str())
            .filter(|scope| groups.group.contains_key(*scope));
        match group {
            Some(group) => *headers.entry(group).or_default() += 1,
            None => found.push(format!("the header `{line}` names no group")),
        }
    }

    for (group, count) in &headers {
        if !touched.contains(*group) {
            found.push(format!(
                "the summary carries a header for `{group}`, which this pull request does not touch"
            ));
        }
        if *count > 1 {
            found.push(format!(
                "the summary carries {count} headers for `{group}`; it takes exactly one"
            ));
        }
    }
    for group in touched {
        if !headers.contains_key(group.as_str()) {
            found.push(format!(
                "the summary carries no header for `{group}`, which this pull request touches"
            ));
        }
    }
    found
}

/// What the check decides from.
pub struct Evidence<'a> {
    pub pull_request: &'a PullRequest,
    pub groups: &'a GroupFile,
    pub changes: &'a [Change],
    /// Versioned manifests whose `[package].version` moved.
    pub version_writes: &'a [String],
    /// Every workspace crate's declared version, by its directory.
    pub versions: &'a BTreeMap<String, String>,
    pub schema_is_current: bool,
}

/// Every clause the pull request fails, as a line naming what to fix.
pub fn violations(evidence: &Evidence<'_>) -> Vec<String> {
    let Evidence {
        pull_request,
        groups,
        changes,
        version_writes,
        versions,
        schema_is_current,
    } = evidence;
    let title = pull_request.title.trim();
    let author = pull_request.user.login.as_str();
    let message = pull_request.message();
    let commit = Commit::parse(&message);

    let mut found = Vec::new();
    match &commit {
        Ok(commit) if !TYPES.contains(&commit.type_().as_str()) => found.push(format!(
            "`{}` is not a type docs/CONTRIBUTING.md lists",
            commit.type_()
        )),
        Ok(_) => {}
        Err(error) => found.push(format!(
            "the title `{title}` is not a conventional commit: {error}"
        )),
    }

    let touched = groups.touched(changes.iter().map(|change| change.path.as_str()));
    let is_summary = commit
        .as_ref()
        .is_ok_and(|commit| commit.type_().as_str() == SUMMARY);
    if is_summary {
        found.extend(summary_violations(
            title,
            &pull_request.description(),
            &touched,
            groups,
        ));
    } else if touched.len() > 1 {
        let touched: Vec<&str> = touched.iter().map(String::as_str).collect();
        found.push(format!(
            "this pull request touches `{}`, so it lands as `summary: …` with one header per group",
            touched.join("`, `")
        ));
    }

    if author != RELEASE_BOT {
        let reserved = groups
            .bookkeeping
            .bot_subjects
            .iter()
            .find(|subject| title.starts_with(subject.as_str()));
        if let Some(subject) = reserved {
            found.push(format!(
                "`{subject}` is reserved to {RELEASE_BOT}; this pull request is by {author}"
            ));
        }
        for manifest in *version_writes {
            found.push(format!(
                "`{manifest}` changes `[package].version`; only {RELEASE_BOT} writes versions"
            ));
        }
    }

    found.extend(lockstep_violations(groups, versions));

    if !commit.as_ref().is_ok_and(declares_covenant_break) {
        let rewritten = changes
            .iter()
            .filter(|change| is_fixture(&change.path) && change.status != Status::Added);
        for change in rewritten {
            found.push(format!(
                "`{}` is a covenant fixture, modified or removed without the `Covenant: break` trailer",
                change.path
            ));
        }
    }

    if !schema_is_current {
        found.push(format!(
            "{SCHEMA_FILE} is stale; run `cargo xtask release-groups schema`"
        ));
    }

    found
}

/// Gathers the evidence from the checkout and the event `payload`, then checks
/// it. A group file that does not describe the workspace refuses here.
pub fn run(metadata: &Metadata, payload: &str, base: &str) -> Result<Vec<String>> {
    let groups = release_groups::load(metadata)?;
    let pull_request = PullRequest::from_event(payload)?;
    let changes = changes(base)?;
    let version_writes = version_writes(&groups, &changes, base)?;
    let committed = fs::read_to_string(metadata.workspace_root.join(SCHEMA_FILE));
    let schema_is_current = committed.is_ok_and(|committed| {
        release_groups::schema().is_ok_and(|generated| committed == generated)
    });

    Ok(violations(&Evidence {
        pull_request: &pull_request,
        groups: &groups,
        changes: &changes,
        version_writes: &version_writes,
        versions: &declared_versions(metadata),
        schema_is_current,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::releases::index::fixture::Repo;
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

        [group.domain]
        members = [ "crates/domain" ]
        anchor = "crates/domain"
        tag = "domain-v{version}"
        breaking = "header"
        floors = [ "cargo-semver-checks" ]
        fixtures = "covenant/fixtures/domain/"
        minted = [ "cut" ]

        [group.outpost]
        members = [ "crates/outpost" ]
        anchor = "crates/outpost"
        tag = "outpost-v{version}"
        breaking = "header"
        floors = [ "cargo-semver-checks" ]
        fixtures = "covenant/fixtures/outpost/"
        minted = [ "cut" ]

        [group.waypoints]
        members = [ "src-tauri", "crates/cartograph" ]
        internals = [ "crates/store" ]
        anchor = "src-tauri"
        tag = "v{version}"
        breaking = "trailer:Covenant: break"
        floors = [ "covenant-fixtures" ]
        fixtures = "covenant/fixtures/waypoints/"
        minted = [ "cut" ]

        [enforce]
        commits = "conventional"
        summary-type = "summary"
        version-writes = "botswain"
        fixtures = "append-only"
        schema = "current"
    "#;

    fn groups() -> GroupFile {
        GroupFile::parse(GROUPS).unwrap()
    }

    fn pull_request(title: &str, body: &str, author: &str) -> PullRequest {
        PullRequest {
            title: title.to_owned(),
            body: Some(body.to_owned()),
            user: User {
                login: author.to_owned(),
            },
        }
    }

    fn modified(path: &str) -> Change {
        Change {
            status: Status::Modified,
            path: path.to_owned(),
        }
    }

    fn added(path: &str) -> Change {
        Change {
            status: Status::Added,
            path: path.to_owned(),
        }
    }

    fn check(pull_request: &PullRequest, changes: &[Change]) -> Vec<String> {
        violations(&Evidence {
            pull_request,
            groups: &groups(),
            changes,
            version_writes: &[],
            versions: &BTreeMap::new(),
            schema_is_current: true,
        })
    }

    fn by_a_person(title: &str, body: &str, changes: &[Change]) -> Vec<String> {
        check(&pull_request(title, body, "jmuchovej"), changes)
    }

    fn assert_names(found: &[String], names: &[&str]) {
        assert!(
            found
                .iter()
                .any(|line| names.iter().all(|name| line.contains(name))),
            "no violation names {names:?}: {found:?}"
        );
    }

    #[test]
    fn a_free_form_title_fails() {
        let found = by_a_person("Fix sync bug", "", &[modified("docs/a.md")]);
        assert_names(&found, &["Fix sync bug", "conventional"]);
    }

    #[test]
    fn a_type_contributing_does_not_list_fails() {
        let found = by_a_person("merge: channel-flavors", "", &[modified("docs/a.md")]);
        assert_names(&found, &["merge"]);
    }

    /// The path decides the group, so a single group's scope stays free-form.
    #[test]
    fn a_single_group_keeps_a_free_form_scope() {
        let changes = [
            modified("crates/store/src/db.rs"),
            modified("src-tauri/src/lib.rs"),
        ];
        assert_eq!(
            by_a_person("refactor(app,store/sync): move it", "", &changes),
            Vec::<String>::new()
        );
    }

    #[test]
    fn two_groups_without_a_summary_fail_naming_both() {
        let changes = [
            modified("crates/bedrock/src/lib.rs"),
            modified("crates/domain/src/lib.rs"),
        ];
        let found = by_a_person("refactor(domain,bedrock): rename", "", &changes);
        assert_names(&found, &["bedrock", "domain", "summary"]);
    }

    #[test]
    fn a_complete_summary_passes() {
        let changes = [
            modified("crates/bedrock/src/lib.rs"),
            modified("crates/outpost/src/lib.rs"),
        ];
        let body = "Why this moved together.\n\nfeat(bedrock)!: rename Filter::eq\nrefactor(outpost): follow the rename\n";
        assert_eq!(
            by_a_person("summary: rename the filter", body, &changes),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_summary_missing_a_group_names_it() {
        let changes = [
            modified("crates/bedrock/src/lib.rs"),
            modified("crates/domain/src/lib.rs"),
        ];
        let found = by_a_person("summary: rename", "feat(bedrock)!: rename", &changes);
        assert_names(&found, &["no header", "domain"]);
    }

    #[test]
    fn a_summary_naming_an_untouched_group_names_it() {
        let changes = [modified("crates/bedrock/src/lib.rs")];
        let body = "feat(bedrock): rename\nrefactor(outpost): follow";
        let found = by_a_person("summary: rename", body, &changes);
        assert_names(&found, &["outpost", "does not touch"]);
    }

    #[test]
    fn a_group_with_two_headers_fails() {
        let changes = [modified("crates/bedrock/src/lib.rs")];
        let body = "feat(bedrock): rename\nfix(bedrock): typo";
        let found = by_a_person("summary: rename", body, &changes);
        assert_names(&found, &["2 headers", "bedrock"]);
    }

    /// `summary(x):` and `summary!:` would slip past cliff's `^summary:` skip,
    /// and a `!` over the whole subject was rejected in favour of per-group
    /// headers.
    #[test]
    fn a_summary_subject_takes_neither_scope_nor_bang() {
        let changes = [modified("crates/bedrock/src/lib.rs")];
        for title in ["summary(bedrock): rename", "summary!: rename"] {
            let found = by_a_person(title, "feat(bedrock): rename", &changes);
            assert_names(&found, &[title]);
        }
    }

    #[test]
    fn a_header_scoped_to_no_group_fails() {
        let changes = [modified("crates/bedrock/src/lib.rs")];
        for header in [
            "feat: rename",
            "feat(store): rename",
            "feat(bedrock,outpost): rename",
        ] {
            let body = format!("feat(bedrock): rename\n{header}");
            let found = by_a_person("summary: rename", &body, &changes);
            assert_names(&found, &[header, "names no group"]);
        }
    }

    /// Prose that merely has a colon in it is not a header.
    #[test]
    fn a_summary_body_may_carry_prose() {
        let changes = [modified("crates/bedrock/src/lib.rs")];
        let body = "Note: this took a while.\n\nfeat(bedrock): rename\n\nSigned-off-by: someone";
        assert_eq!(
            by_a_person("summary: rename", body, &changes),
            Vec::<String>::new()
        );
    }

    /// GitHub's web form submits CRLF.
    #[test]
    fn a_crlf_description_reads_the_same() {
        let changes = [
            modified("crates/bedrock/src/lib.rs"),
            modified("crates/domain/src/lib.rs"),
        ];
        let body = "feat(bedrock): rename\r\nfix(domain): follow\r\n";
        assert_eq!(
            by_a_person("summary: rename", body, &changes),
            Vec::<String>::new()
        );
        let fixture = [modified("covenant/fixtures/domain/a.json")];
        assert_eq!(
            by_a_person(
                "fix(domain): reread it",
                "Why.\r\n\r\nCovenant: break\r\n",
                &fixture
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn an_empty_description_is_null_in_the_payload() {
        let payload = r#"{ "pull_request": {
            "title": "docs: a note", "body": null, "user": { "login": "jmuchovej" }
        } }"#;
        let parsed = PullRequest::from_event(payload).unwrap();
        assert_eq!(check(&parsed, &[]), Vec::<String>::new());
        assert_eq!(parsed.message(), "docs: a note");
    }

    #[test]
    fn a_payload_without_a_pull_request_is_refused() {
        assert!(PullRequest::from_event(r#"{ "ref": "refs/heads/main" }"#).is_err());
    }

    #[test]
    fn the_bookkeeping_subject_is_reserved_to_the_bot() {
        let changes = [modified("nix/releases/nightly.json")];
        let title = "chore(release): record v0.2.0";
        assert_names(
            &by_a_person(title, "", &changes),
            &["chore(release):", RELEASE_BOT, "jmuchovej"],
        );
        assert_eq!(
            check(&pull_request(title, "", RELEASE_BOT), &changes),
            Vec::<String>::new()
        );
    }

    fn with_version_writes(author: &str, writes: &[String]) -> Vec<String> {
        violations(&Evidence {
            pull_request: &pull_request("fix(domain): a bug", "", author),
            groups: &groups(),
            changes: &[modified("crates/domain/Cargo.toml")],
            version_writes: writes,
            versions: &BTreeMap::new(),
            schema_is_current: true,
        })
    }

    #[test]
    fn a_hand_edited_version_fails_naming_the_file() {
        let writes = ["crates/domain/Cargo.toml".to_owned()];
        assert_names(
            &with_version_writes("jmuchovej", &writes),
            &["crates/domain/Cargo.toml", "[package].version"],
        );
    }

    #[test]
    fn the_release_bot_writes_versions() {
        let writes = ["crates/domain/Cargo.toml".to_owned()];
        assert_eq!(
            with_version_writes(RELEASE_BOT, &writes),
            Vec::<String>::new()
        );
    }

    fn writes_between(before: &str, after: &str, changes: &[Change]) -> Vec<String> {
        differing_versions(&groups(), changes, |_| {
            Ok((before.to_owned(), after.to_owned()))
        })
        .unwrap()
    }

    #[test]
    fn a_member_s_package_version_is_a_write() {
        let changes = [modified("crates/domain/Cargo.toml")];
        assert_eq!(
            writes_between(
                "[package]\nversion = \"0.1.0\"\n",
                "[package]\nversion = \"0.2.0\"\n",
                &changes
            ),
            ["crates/domain/Cargo.toml"]
        );
    }

    /// Dependencies carry `version` too.
    #[test]
    fn a_dependency_s_version_is_not_a_write() {
        let changes = [modified("crates/domain/Cargo.toml")];
        let manifest = |serde: &str| {
            format!(
                "[package]\nversion = \"0.1.0\"\n[dependencies]\nserde = {{ version = \"{serde}\" }}\n"
            )
        };
        assert!(writes_between(&manifest("1.0"), &manifest("1.1"), &changes).is_empty());
    }

    /// `store` is an internal, and a new crate's manifest writes no version
    /// over an earlier one. `tooling/` is not named at all.
    #[test]
    fn only_a_modified_member_manifest_is_read() {
        let changes = [
            modified("crates/store/Cargo.toml"),
            modified("tooling/xtask/Cargo.toml"),
            added("crates/domain/Cargo.toml"),
        ];
        assert!(
            differing_versions(&groups(), &changes, |path| {
                panic!("{path} is no versioned member's existing manifest")
            })
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn adding_a_fixture_passes() {
        let changes = [added("covenant/fixtures/domain/a.json")];
        assert_eq!(
            by_a_person("test(domain): freeze a", "", &changes),
            Vec::<String>::new()
        );
    }

    #[test]
    fn modifying_a_fixture_without_the_trailer_names_it() {
        let changes = [modified("covenant/fixtures/domain/a.json")];
        let found = by_a_person(
            "fix(domain): reread it",
            "Mentions Covenant: break inline.",
            &changes,
        );
        assert_names(
            &found,
            &["covenant/fixtures/domain/a.json", "Covenant: break"],
        );
    }

    #[test]
    fn modifying_a_fixture_with_the_trailer_passes() {
        let changes = [modified("covenant/fixtures/domain/a.json")];
        let body = "The old reading was wrong.\n\nCovenant: break\n";
        assert_eq!(
            by_a_person("fix(domain): reread it", body, &changes),
            Vec::<String>::new()
        );
    }

    #[test]
    fn removing_a_fixture_fails() {
        let changes = [Change {
            status: Status::Deleted,
            path: "covenant/fixtures/domain/a.json".to_owned(),
        }];
        assert_names(
            &by_a_person("fix(domain): drop it", "", &changes),
            &["covenant/fixtures/domain/a.json"],
        );
    }

    #[test]
    fn a_group_s_readme_is_not_a_fixture() {
        let changes = [modified("covenant/fixtures/domain/README.md")];
        assert_eq!(
            by_a_person("docs(domain): fix a typo", "", &changes),
            Vec::<String>::new()
        );
    }

    /// Import corpora are Markdown, so this one is data.
    #[test]
    fn a_readme_inside_a_corpus_is_a_fixture() {
        let path = "covenant/fixtures/transload/vault/README.md";
        let found = by_a_person("fix(transload): reread it", "", &[modified(path)]);
        assert_names(&found, &[path, "Covenant: break"]);
    }

    /// Real git, since this is what renames and path quoting come down to.
    #[test]
    fn git_reports_a_renamed_fixture_as_removed_and_keeps_paths_literal() {
        let repo = Repo::new();
        repo.commit("covenant/fixtures/domain/a.json", "add a fixture");
        repo.git(&[
            "mv",
            "covenant/fixtures/domain/a.json",
            "covenant/fixtures/domain/é.json",
        ]);
        repo.git(&["commit", "-q", "-m", "rename it"]);

        let changes = parse_changes(&repo.git(&diff_args("HEAD~1")));

        assert_eq!(
            changes,
            [
                Change {
                    status: Status::Deleted,
                    path: "covenant/fixtures/domain/a.json".to_owned()
                },
                added("covenant/fixtures/domain/é.json"),
            ]
        );
        assert_names(
            &by_a_person("fix(domain): rename it", "", &changes),
            &["covenant/fixtures/domain/a.json"],
        );
    }

    #[test]
    fn a_stale_schema_fails() {
        let found = violations(&Evidence {
            pull_request: &pull_request("docs: a note", "", "jmuchovej"),
            groups: &groups(),
            changes: &[],
            version_writes: &[],
            versions: &BTreeMap::new(),
            schema_is_current: false,
        });
        assert_names(&found, &[SCHEMA_FILE, "release-groups schema"]);
    }

    fn versions(declared: &[(&str, &str)]) -> BTreeMap<String, String> {
        declared
            .iter()
            .map(|(dir, version)| ((*dir).to_owned(), (*version).to_owned()))
            .collect()
    }

    #[test]
    fn a_drifted_member_fails_naming_both_manifests() {
        let declared = versions(&[("src-tauri", "0.2.0"), ("crates/cartograph", "0.1.9")]);
        assert_names(
            &lockstep_violations(&groups(), &declared),
            &["crates/cartograph/Cargo.toml", "src-tauri/Cargo.toml"],
        );
    }

    /// The prerelease is part of the version: `main`'s `-nightly` is mirrored.
    #[test]
    fn a_member_missing_the_anchor_s_prerelease_has_drifted() {
        let declared = versions(&[
            ("src-tauri", "0.1.0-nightly"),
            ("crates/cartograph", "0.1.0"),
        ]);
        assert_names(
            &lockstep_violations(&groups(), &declared),
            &["crates/cartograph/Cargo.toml", "0.1.0-nightly"],
        );
    }

    /// Being the release bot's does not excuse drift, so the planner has to
    /// write every member in one commit.
    #[test]
    fn the_release_bot_setting_every_member_passes_and_setting_one_fails() {
        let writes = [
            "src-tauri/Cargo.toml".to_owned(),
            "crates/cartograph/Cargo.toml".to_owned(),
        ];
        let check_bot_write = |declared: &BTreeMap<String, String>| {
            violations(&Evidence {
                pull_request: &pull_request(
                    "chore(release): v0.2.x preview 0.2.0-pre.2",
                    "",
                    RELEASE_BOT,
                ),
                groups: &groups(),
                changes: &[
                    modified("src-tauri/Cargo.toml"),
                    modified("crates/cartograph/Cargo.toml"),
                ],
                version_writes: &writes,
                versions: declared,
                schema_is_current: true,
            })
        };

        let both = versions(&[
            ("src-tauri", "0.2.0-pre.2"),
            ("crates/cartograph", "0.2.0-pre.2"),
        ]);
        assert_eq!(check_bot_write(&both), Vec::<String>::new());

        let one = versions(&[
            ("src-tauri", "0.2.0-pre.2"),
            ("crates/cartograph", "0.2.0-pre.1"),
        ]);
        assert_names(
            &check_bot_write(&one),
            &["crates/cartograph/Cargo.toml", "src-tauri/Cargo.toml"],
        );
    }

    /// `store` is an internal, so it carries nothing to compare.
    #[test]
    fn an_internal_is_not_held_to_its_anchor() {
        let declared = versions(&[
            ("src-tauri", "0.2.0"),
            ("crates/cartograph", "0.2.0"),
            ("crates/store", "0.0.0"),
        ]);
        assert_eq!(
            lockstep_violations(&groups(), &declared),
            Vec::<String>::new()
        );
    }

    /// The tree this check runs on has to pass it.
    #[test]
    fn this_workspace_is_in_lockstep() {
        let metadata = crate::workspace::load().unwrap();
        let groups = release_groups::load(&metadata).unwrap();
        assert_eq!(
            lockstep_violations(&groups, &declared_versions(&metadata)),
            Vec::<String>::new()
        );
    }

    /// Reports every clause, so one push fixes them all.
    #[test]
    fn every_failing_clause_is_reported() {
        let changes = [
            modified("crates/bedrock/src/lib.rs"),
            modified("crates/domain/src/lib.rs"),
            modified("covenant/fixtures/domain/a.json"),
        ];
        assert_eq!(by_a_person("Fix sync bug", "", &changes).len(), 3);
    }

    fn repository_file(path: &str) -> String {
        fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    /// CONTRIBUTING is the contract: what it lists is what the check accepts.
    #[test]
    fn the_accepted_types_are_the_ones_contributing_lists() {
        let contributing = repository_file("docs/CONTRIBUTING.md");
        let table = contributing
            .split("\n## Types\n")
            .nth(1)
            .and_then(|section| section.split("\n## ").next())
            .expect("CONTRIBUTING has a Types section");
        let listed: Vec<&str> = table
            .lines()
            .filter_map(|line| line.strip_prefix("| `")?.split('`').next())
            .collect();
        assert_eq!(listed, TYPES);
        assert!(contributing.contains("Covenant: break"));
    }

    /// A summary's per-group headers live in a body cliff does not parse as
    /// subjects, so the subject is the only line it would render.
    #[test]
    fn release_notes_skip_a_summary() {
        let cliff = repository_file(".config/cliff.toml");
        assert!(
            cliff
                .lines()
                .any(|line| line.contains("^summary:") && line.contains("skip = true")),
            "cliff.toml no longer skips a summary"
        );
    }
}
