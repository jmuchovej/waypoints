// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The one history walk both planners read: `main`'s first-parent commits,
//! each attributed to the release groups it counts for.
//!
//! A pull request lands as a merge commit, so the merge commit is the unit:
//! its message, and its diff against its first parent. The branch commits
//! beneath it are never read. A library group counts a commit that changes
//! its own paths; the app counts its own paths and every group's, since the
//! binary ships them. Bookkeeping counts for nothing: the release index, the
//! treewide passes `.git-blame-ignore-revs` lists, and the release bot's own
//! subjects.

use crate::{release_groups::GroupFile, tools::git};
use anyhow::{Context as _, Result};
use std::{collections::BTreeSet, fs, path::Path};

/// Record and field separators the format emits; no message or path carries
/// either.
const RECORD: char = '\x1e';
const FIELD: char = '\x1f';

/// The trailer whose value marks a covenant break.
const COVENANT: &str = "Covenant";

/// One first-parent commit, attributed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    pub subject: String,
    /// Everything after the subject, trailers included.
    pub body: String,
    /// Whether a `Covenant: break` trailer is present.
    pub covenant_break: bool,
    /// The paths changed against the first parent, bookkeeping included.
    pub paths: Vec<String>,
    /// The groups this commit counts for: empty for bookkeeping, tooling, and
    /// ignored revisions.
    pub groups: BTreeSet<String>,
}

impl Commit {
    pub fn counts_for(&self, group: &str) -> bool {
        self.groups.contains(group)
    }

    /// The message as a conventional-commit parser reads it.
    pub fn message(&self) -> String {
        if self.body.is_empty() {
            self.subject.clone()
        } else {
            format!("{}\n\n{}", self.subject, self.body)
        }
    }
}

/// A record as the format emits it, before attribution.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Raw {
    pub(crate) sha: String,
    pub(crate) subject: String,
    pub(crate) body: String,
    pub(crate) covenant_break: bool,
    pub(crate) paths: Vec<String>,
}

/// `git log` over `range`: first-parent, oldest first, one record per commit
/// followed by the paths it changed against its first parent. The format ends
/// in a field separator so the path list `--name-only` appends is set apart
/// from the trailer value before it.
pub fn log_args(range: &str) -> Vec<String> {
    vec![
        "log".to_owned(),
        "--first-parent".to_owned(),
        "--diff-merges=first-parent".to_owned(),
        "--reverse".to_owned(),
        "--name-only".to_owned(),
        format!(
            "--format={RECORD}%H{FIELD}%s{FIELD}%b{FIELD}%(trailers:key={COVENANT},valueonly){FIELD}"
        ),
        range.to_owned(),
    ]
}

pub(crate) fn parse(output: &str) -> Vec<Raw> {
    output
        .split(RECORD)
        .filter(|record| !record.trim().is_empty())
        .map(|record| {
            let mut fields = record.splitn(5, FIELD);
            let mut next = || fields.next().unwrap_or_default();
            let sha = next().trim().to_owned();
            let subject = next().trim().to_owned();
            let body = next().trim().to_owned();
            let covenant_break = next().lines().any(|line| line.trim() == "break");
            let paths = next()
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect();
            Raw {
                sha,
                subject,
                body,
                covenant_break,
                paths,
            }
        })
        .collect()
}

/// Which groups `raw` counts for. A revision the ignore file lists or a
/// subject reserved to the release bot counts for none; otherwise each
/// product path's owner counts, and when any does, so does every group that
/// counts the closure.
pub(crate) fn attribute(raw: Raw, groups: &GroupFile, ignored: &BTreeSet<String>) -> Commit {
    let bookkeeping = ignored.iter().any(|rev| raw.sha.starts_with(rev.as_str()))
        || groups
            .bookkeeping
            .bot_subjects
            .iter()
            .any(|subject| raw.subject.starts_with(subject.as_str()));

    let mut counted = BTreeSet::new();
    if !bookkeeping {
        for path in &raw.paths {
            if let Some(owner) = groups.owner_of(path) {
                counted.insert(owner.to_owned());
            }
        }
        if !counted.is_empty() {
            counted.extend(groups.closure_groups().map(str::to_owned));
        }
    }

    Commit {
        sha: raw.sha,
        subject: raw.subject,
        body: raw.body,
        covenant_break: raw.covenant_break,
        paths: raw.paths,
        groups: counted,
    }
}

/// The revisions `groups`' ignore file lists under `root`: one full commit id
/// per line, comments and blanks skipped.
pub fn ignored_revisions(root: &Path, groups: &GroupFile) -> Result<BTreeSet<String>> {
    let path = root.join(&groups.bookkeeping.ignore_revs);
    let text =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

/// Every first-parent commit in `range`, attributed, oldest first.
pub fn walk(range: &str, groups: &GroupFile, ignored: &BTreeSet<String>) -> Result<Vec<Commit>> {
    let args = log_args(range);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = git::run(&args)?;
    Ok(parse(&output)
        .into_iter()
        .map(|raw| attribute(raw, groups, ignored))
        .collect())
}

/// The subjects of the commits that count for any group: the planners' "did
/// anything land" view.
pub fn product_subjects(commits: &[Commit]) -> Vec<String> {
    commits
        .iter()
        .filter(|commit| !commit.groups.is_empty())
        .map(|commit| commit.subject.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::releases::index::{INDEX_DIR, fixture::Repo};

    fn groups(commit: &Commit) -> Vec<&str> {
        commit.groups.iter().map(String::as_str).collect()
    }

    /// The format's records and fields come apart again, paths and all.
    #[test]
    fn the_format_parses_back_into_records() {
        let output = format!(
            "{RECORD}abc123{FIELD}feat(bedrock): a thing{FIELD}A body.\n\nCovenant: break{FIELD}break\n{FIELD}\ncrates/bedrock/src/lib.rs\nsrc-tauri/src/main.rs\n\n\
             {RECORD}def456{FIELD}chore: nothing{FIELD}{FIELD}{FIELD}\n"
        );
        let records = parse(&output);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sha, "abc123");
        assert_eq!(records[0].subject, "feat(bedrock): a thing");
        assert_eq!(records[0].body, "A body.\n\nCovenant: break");
        assert!(records[0].covenant_break);
        assert_eq!(
            records[0].paths,
            ["crates/bedrock/src/lib.rs", "src-tauri/src/main.rs"]
        );
        assert_eq!(records[1].subject, "chore: nothing");
        assert!(!records[1].covenant_break);
        assert!(records[1].body.is_empty() && records[1].paths.is_empty());
    }

    /// Scenario _Library-only change_: the owning group and the app, nothing
    /// else.
    #[test]
    fn a_library_change_counts_for_its_group_and_the_app() {
        let repo = Repo::new();
        repo.commit("crates/bedrock-core/src/lib.rs", "feat(core): a thing");
        let walked = repo.walk("HEAD");
        assert_eq!(walked.len(), 1);
        assert_eq!(groups(&walked[0]), ["bedrock", "waypoints"]);
        assert!(!walked[0].counts_for("outpost") && !walked[0].counts_for("domain"));
    }

    /// Scenario _Tooling-only change_.
    #[test]
    fn a_tooling_change_counts_for_no_group() {
        let repo = Repo::new();
        repo.commit("tooling/xtask/src/lib.rs", "feat(xtask): a thing");
        let walked = repo.walk("HEAD");
        assert!(walked[0].groups.is_empty(), "{:?}", walked[0]);
        assert_eq!(repo.product_commits("HEAD"), Vec::<String>::new());
    }

    /// Scenario _Treewide formatting pass_: a revision the ignore file lists
    /// counts for nothing, whatever it touched.
    #[test]
    fn an_ignored_revision_counts_for_no_group() {
        let repo = Repo::new();
        repo.product("feat: first");
        repo.commit("crates/bedrock/src/lib.rs", "style: format everything");
        repo.ignore(&repo.head());
        let walked = repo.walk("HEAD");
        assert_eq!(walked.len(), 2);
        assert_eq!(groups(&walked[0]), ["domain", "waypoints"]);
        assert!(walked[1].groups.is_empty(), "{:?}", walked[1]);
        assert_eq!(repo.product_commits("HEAD"), ["feat: first"]);
    }

    /// Scenario _Release bot bookkeeping_: the reserved subject counts for
    /// nothing, even where it rewrote a manifest.
    #[test]
    fn the_bot_s_bookkeeping_counts_for_no_group() {
        let repo = Repo::new();
        repo.commit("crates/bedrock/Cargo.toml", "chore(release): bedrock 0.2.0");
        let walked = repo.walk("HEAD");
        assert!(walked[0].groups.is_empty(), "{:?}", walked[0]);
    }

    /// Scenario _A pull request lands as one commit_: the merge commit's
    /// message, the union of its branch's paths, and nothing from beneath.
    #[test]
    fn a_pull_request_lands_as_one_commit() {
        let repo = Repo::new();
        repo.product("feat: before");
        repo.branch("topic");
        repo.commit("crates/bedrock/src/lib.rs", "wip: one");
        repo.commit("crates/bedrock/src/query.rs", "wip: two");
        repo.commit("src-tauri/src/main.rs", "wip: three");
        repo.merge(
            "topic",
            "summary: the filter API\n\nfeat(bedrock)!: rename the filter API\nrefactor(waypoints): follow the rename",
        );

        let walked = repo.walk("HEAD");
        assert_eq!(walked.len(), 2, "{walked:?}");
        let landed = &walked[1];
        assert_eq!(landed.subject, "summary: the filter API");
        assert!(
            landed
                .body
                .contains("feat(bedrock)!: rename the filter API")
        );
        assert_eq!(
            landed.paths,
            [
                "crates/bedrock/src/lib.rs",
                "crates/bedrock/src/query.rs",
                "src-tauri/src/main.rs",
            ]
        );
        assert_eq!(groups(landed), ["bedrock", "waypoints"]);
        assert!(
            walked
                .iter()
                .all(|commit| !commit.subject.starts_with("wip:")),
            "{walked:?}"
        );
    }

    /// The trailer is read from the merge message, not guessed from the
    /// subject.
    #[test]
    fn the_covenant_trailer_is_read() {
        let repo = Repo::new();
        repo.commit_with(
            "crates/domain/src/lib.rs",
            "feat(domain): widen the envelope\n\nCovenant: break",
        );
        repo.commit_with(
            "crates/domain/src/other.rs",
            "fix(domain): a bug\n\nNo trailer here.",
        );
        let walked = repo.walk("HEAD");
        assert!(walked[0].covenant_break, "{:?}", walked[0]);
        assert!(!walked[1].covenant_break, "{:?}", walked[1]);
        assert_eq!(
            walked[0].message(),
            "feat(domain): widen the envelope\n\nCovenant: break"
        );
    }

    /// The exclusion the index relies on is stated in the group file; this
    /// keeps it the same directory the index writes to.
    #[test]
    fn the_index_directory_is_bookkeeping_in_this_workspace() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = fs::read_to_string(root.join(crate::release_groups::GROUP_FILE)).unwrap();
        let file = GroupFile::parse(&text).unwrap();
        assert!(
            file.bookkeeping.paths.contains(&format!("{INDEX_DIR}/")),
            "{:?}",
            file.bookkeeping.paths
        );
        assert!(file.is_bookkeeping(&format!("{INDEX_DIR}/nightly.json")));
    }
}
