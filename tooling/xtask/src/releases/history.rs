// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The one history walk both planners read: `main`'s first-parent commits,
//! each attributed to the release groups it counts for.
//!
//! A pull request lands as a merge commit, so the merge commit is the unit:
//! its message, and its diff against its first parent. The branch commits
//! beneath it are never read. A commit counts for exactly the groups whose
//! owned paths it changes; a path no group lists counts for none. Bookkeeping
//! counts for nothing: the release index, the treewide passes
//! `.git-blame-ignore-revs` lists, and the release bot's own subjects.

use crate::{
    constants::{APP_GROUP, BLAME_IGNORE_FILE, RELEASE_BOT},
    release_groups::GroupFile,
    tools::git,
};
use anyhow::{Context as _, Result};
use git_conventional::{Commit as Conventional, Type};
use indoc::formatdoc;
use std::{collections::BTreeSet, fs, path::Path};

/// Record and field separators the format emits; no message or path carries
/// either.
const RECORD: char = '\x1e';
const FIELD: char = '\x1f';

/// The trailer whose value marks a covenant break.
const COVENANT: &str = "Covenant";

/// The multi-group subject type: the body carries one header per group.
const SUMMARY: &str = "summary";

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
        formatdoc! { "
            {subject}

            {body}
          ", subject = self.subject, body = self.body}
        .trim()
        .to_string()
    }

    /// The header `group` reads this commit by, as the version bumper takes
    /// it, or none where the commit counts for another group only.
    ///
    /// A summary's subject counts for no group: a library reads the body
    /// header scoped to it, and the app reads the highest-ranked header. A
    /// single-group commit is read by its subject. A library breaks on `!` in
    /// its own header; the app never does, and its one breaking signal is the
    /// trailer, which is re-expressed as `!` for the bumper. A group the file
    /// does not name has no header.
    pub fn header_for(&self, group: &str, groups: &GroupFile) -> Option<String> {
        if !self.counts_for(group) || !groups.group.contains_key(group) {
            return None;
        }
        let subject = Conventional::parse(&self.subject).ok();
        let is_summary = subject
            .as_ref()
            .is_some_and(|commit| commit.type_().as_str() == SUMMARY);

        if group == APP_GROUP {
            let declared = if is_summary {
                highest(self.body_headers())
            } else {
                subject
            };
            return Some(match declared {
                Some(header) => format!(
                    "{kind}{bang}: {description}",
                    kind = header.type_(),
                    bang = if self.covenant_break { "!" } else { "" },
                    description = header.description(),
                ),
                // Not conventional: the bumper decides what that is worth.
                None => self.subject.clone(),
            });
        }
        if is_summary {
            self.body
                .lines()
                .find(|line| {
                    Conventional::parse(line).is_ok_and(|header| {
                        header.type_().as_str() != SUMMARY
                            && header.scope().is_some_and(|scope| scope.as_str() == group)
                    })
                })
                .map(str::to_owned)
        } else {
            Some(self.subject.clone())
        }
    }

    /// The conventional headers in the body, in order, the summary line aside.
    fn body_headers(&self) -> Vec<Conventional<'_>> {
        self.body
            .lines()
            .filter_map(|line| Conventional::parse(line).ok())
            .filter(|header| header.type_().as_str() != SUMMARY)
            .collect()
    }
}

/// The header that claims the most: a feature over a fix over anything
/// else, the first of equals.
fn highest(headers: Vec<Conventional<'_>>) -> Option<Conventional<'_>> {
    let mut best: Option<Conventional<'_>> = None;
    for header in headers {
        if best
            .as_ref()
            .is_none_or(|current| rank(header.type_()) > rank(current.type_()))
        {
            best = Some(header);
        }
    }
    best
}

fn rank(kind: Type<'_>) -> u8 {
    if kind == Type::FEAT {
        2
    } else if kind == Type::FIX {
        1
    } else {
        0
    }
}

/// A record as the format emits it, before attribution.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Raw {
    pub(crate) sha: String,
    pub(crate) author: String,
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
            "--format={RECORD}%H{FIELD}%an{FIELD}%s{FIELD}%b{FIELD}%(trailers:key={COVENANT},valueonly){FIELD}"
        ),
        range.to_owned(),
    ]
}

pub(crate) fn parse(output: &str) -> Vec<Raw> {
    output
        .split(RECORD)
        .filter(|record| !record.trim().is_empty())
        .map(|record| {
            let mut fields = record.splitn(6, FIELD);
            let mut next = || fields.next().unwrap_or_default();
            let sha = next().trim().to_owned();
            let author = next().trim().to_owned();
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
                author,
                subject,
                body,
                covenant_break,
                paths,
            }
        })
        .collect()
}

/// Which groups `raw` counts for. A revision in `ignored`, a commit the
/// release bot authored, or a subject reserved to it counts for none;
/// otherwise exactly the owners of the paths it changed.
pub(crate) fn attribute(raw: Raw, groups: &GroupFile, ignored: &BTreeSet<String>) -> Commit {
    let bookkeeping = ignored.iter().any(|rev| raw.sha.starts_with(rev.as_str()))
        || raw.author == RELEASE_BOT
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

/// The revisions that count for nothing: those [`BLAME_IGNORE_FILE`] lists
/// under `root`, one full commit id per line with comments and blanks
/// skipped, and those `groups` names in `[bookkeeping].ignore-revs`.
pub fn ignored_revisions(root: &Path, groups: &GroupFile) -> Result<BTreeSet<String>> {
    let path = root.join(BLAME_IGNORE_FILE);
    let text =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .chain(groups.bookkeeping.ignore_revs.iter().cloned())
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
    use indoc::indoc;

    use super::*;
    use crate::{
        constants::INDEX_DIR,
        releases::{index::fixture::Repo, version::Version},
    };

    fn groups(commit: &Commit) -> Vec<&str> {
        commit.groups.iter().map(String::as_str).collect()
    }

    /// The format's records and fields come apart again, paths and all.
    #[test]
    fn the_format_parses_back_into_records() {
        let output = format!(
            "{RECORD}abc123{FIELD}John{FIELD}feat(bedrock): a thing{FIELD}A body.\n\nCovenant: break{FIELD}break\n{FIELD}\ncrates/bedrock/src/lib.rs\nsrc-tauri/src/main.rs\n\n\
             {RECORD}def456{FIELD}xtask{FIELD}chore: nothing{FIELD}{FIELD}{FIELD}\n"
        );
        let records = parse(&output);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sha, "abc123");
        assert_eq!(records[0].author, "John");
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

    /// Scenario _Library-only change_: the owning group, and nothing else —
    /// not the app, which ships the library but did not change.
    #[test]
    fn a_library_change_counts_for_its_group_alone() {
        let repo = Repo::new();
        repo.commit("crates/bedrock-core/src/lib.rs", "feat(core): a thing");
        let walked = repo.walk("HEAD");
        assert_eq!(walked.len(), 1);
        assert_eq!(groups(&walked[0]), ["bedrock"]);
        assert!(!walked[0].counts_for("waypoints"), "{:?}", walked[0]);
    }

    /// Scenario _Frontend change_: `src/` is listed under the app, so it
    /// counts for the app alone.
    #[test]
    fn a_frontend_change_counts_for_the_app() {
        let repo = Repo::new();
        repo.commit("src/app/editor.vue", "feat(app/editor): a thing");
        let walked = repo.walk("HEAD");
        assert_eq!(groups(&walked[0]), ["waypoints"]);
    }

    /// Scenario _Unlisted path_: a path no group lists counts for none, so a
    /// day of such commits is no product work and publishes no nightly.
    #[test]
    fn an_unlisted_path_counts_for_no_group() {
        let repo = Repo::new();
        repo.commit("website/content/index.md", "docs(website): a page");
        repo.commit("docs/CONTRIBUTING.md", "docs: a rule");
        repo.commit("distribution/nix/overlay.nix", "build(nix): a thing");
        let walked = repo.walk("HEAD");
        assert!(
            walked.iter().all(|commit| commit.groups.is_empty()),
            "{walked:?}"
        );
        assert_eq!(repo.product_commits("HEAD"), Vec::<String>::new());
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

    /// Scenario _Treewide formatting pass_: a revision the blame file lists
    /// counts for nothing, whatever it touched.
    #[test]
    fn an_ignored_revision_counts_for_no_group() {
        let repo = Repo::new();
        repo.product("feat: first");
        repo.commit("crates/bedrock/src/lib.rs", "style: format everything");
        repo.ignore(&repo.head());
        let walked = repo.walk("HEAD");
        assert_eq!(walked.len(), 2);
        assert_eq!(groups(&walked[0]), ["domain"]);
        assert!(walked[1].groups.is_empty(), "{:?}", walked[1]);
        assert_eq!(repo.product_commits("HEAD"), ["feat: first"]);
    }

    /// Scenario _A listed revision counts for nothing_: a revision the group
    /// file names counts for nothing, whatever it touched — and, unlike one
    /// the blame file lists, keeps its blame.
    #[test]
    fn a_listed_revision_counts_for_no_group() {
        let repo = Repo::new();
        repo.product("feat: first");
        repo.commit(
            "crates/domain/docs/dialect.md",
            "refactor(importer)!: delete the dormant crate",
        );
        let listing = crate::releases::index::fixture::GROUPS.replace(
            "bot-subjects",
            &format!(
                "ignore-revs = [ \"{}\" ]\n        bot-subjects",
                repo.head()
            ),
        );
        let walked = repo.walk_under("HEAD", &listing);
        assert_eq!(walked.len(), 2);
        assert_eq!(groups(&walked[0]), ["domain"]);
        assert!(walked[1].groups.is_empty(), "{:?}", walked[1]);
        assert_eq!(product_subjects(&walked), ["feat: first"]);
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

    /// Whatever the release bot authors is bookkeeping: a cut's summary
    /// lands under its name with headers scoped by group, and must not read
    /// as the next cut's work.
    #[test]
    fn the_release_bot_s_commits_count_for_no_group() {
        let repo = Repo::new();
        repo.commit_as(
            RELEASE_BOT,
            "crates/bedrock/Cargo.toml",
            indoc! { "
              summary: cut 0.2.0-pre.1

              chore(bedrock): 0.2.0
              chore(waypoints): main now targets 0.3.0-nightly
            " },
        );
        repo.commit_as(
            "someone",
            "crates/bedrock/src/lib.rs",
            "feat(bedrock): after",
        );
        let walked = repo.walk("HEAD");
        assert!(walked[0].groups.is_empty(), "{:?}", walked[0]);
        assert_eq!(groups(&walked[1]), ["bedrock"]);
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
            indoc::indoc! {"
                summary: the filter API

                feat(bedrock)!: rename the filter API
                refactor(waypoints): follow the rename"},
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
        let text = fs::read_to_string(root.join(crate::constants::GROUP_FILE)).unwrap();
        let file = GroupFile::parse(&text).unwrap();
        assert!(
            file.bookkeeping.paths.contains(&format!("{INDEX_DIR}/")),
            "{:?}",
            file.bookkeeping.paths
        );
        assert!(file.is_bookkeeping(&format!("{INDEX_DIR}/nightly.json")));
    }

    fn file() -> GroupFile {
        GroupFile::parse(crate::releases::index::fixture::GROUPS).unwrap()
    }

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn header(commit: &Commit, group: &str) -> Vec<String> {
        commit.header_for(group, &file()).into_iter().collect()
    }

    /// Scenarios _Library break with caller edits elsewhere_ and _Summary
    /// across two libraries_, on a `1.x` base so a break and a feature cannot
    /// look alike: breaking for `bedrock`, non-breaking for `outpost`, and
    /// nothing for the app or for a group the summary does not touch.
    #[test]
    fn a_summary_reads_one_header_per_group() {
        let repo = Repo::new();
        repo.product("feat: before");
        repo.branch("topic");
        repo.commit("crates/bedrock/src/lib.rs", "wip: one");
        repo.commit("crates/outpost/src/lib.rs", "wip: two");
        repo.merge(
            "topic",
            indoc::indoc! {"
                summary: rename the filter API

                feat(bedrock)!: rename the filter API
                refactor(outpost): follow the rename"},
        );
        let landed = repo.walk("HEAD").pop().unwrap();
        assert_eq!(groups(&landed), ["bedrock", "outpost"]);

        let bedrock = header(&landed, "bedrock");
        let outpost = header(&landed, "outpost");
        assert_eq!(bedrock, ["feat(bedrock)!: rename the filter API"]);
        assert_eq!(outpost, ["refactor(outpost): follow the rename"]);
        assert_eq!(header(&landed, "waypoints"), Vec::<String>::new());
        assert_eq!(header(&landed, "domain"), Vec::<String>::new());

        let base = v("1.2.0");
        assert_eq!(base.next_library(&bedrock), v("2.0.0"));
        assert_eq!(base.next_library(&outpost), v("1.2.1"));
    }

    /// A summary touching the app's own paths reads the app's header as the
    /// highest-ranked body header, with no `!`.
    #[test]
    fn a_summary_touching_the_app_reads_its_highest_header() {
        let repo = Repo::new();
        repo.product("feat: before");
        repo.branch("topic");
        repo.commit("crates/bedrock/src/lib.rs", "wip: one");
        repo.commit("src-tauri/src/main.rs", "wip: two");
        repo.merge(
            "topic",
            indoc::indoc! {"
                summary: rename the filter API

                feat(bedrock)!: rename the filter API
                refactor(waypoints): follow the rename"},
        );
        let landed = repo.walk("HEAD").pop().unwrap();
        assert_eq!(groups(&landed), ["bedrock", "waypoints"]);
        let app = header(&landed, "waypoints");
        assert_eq!(app, ["feat: rename the filter API"]);
        assert_eq!(v("1.2.0").cut(&app).unwrap(), v("1.3.0-pre.1"));
    }

    /// Scenario _Single-group commit with a free-form scope_: the path
    /// decides the group, and the scope is kept for the library's bumper;
    /// the app, whose paths it did not touch, reads nothing from it.
    #[test]
    fn a_single_group_commit_keeps_its_free_form_scope() {
        let repo = Repo::new();
        repo.commit("crates/bedrock/src/query.rs", "fix(query): a bug");
        let commit = repo.walk("HEAD").pop().unwrap();

        let bedrock = header(&commit, "bedrock");
        assert_eq!(bedrock, ["fix(query): a bug"]);
        assert_eq!(header(&commit, "waypoints"), Vec::<String>::new());
        assert_eq!(v("1.2.0").next_library(&bedrock), v("1.2.1"));
    }

    /// The app never breaks on `!`, and does break on the trailer, which
    /// reaches the bumper as `!` on the app's own header.
    #[test]
    fn the_app_ignores_the_bang_and_reads_the_trailer() {
        let repo = Repo::new();
        repo.commit_with("src-tauri/src/main.rs", "feat(app)!: drop the old importer");
        repo.commit_with(
            "src-tauri/src/other.rs",
            "fix(app): tighten the envelope\n\nCovenant: break",
        );
        let walked = repo.walk("HEAD");

        let banged = header(&walked[0], "waypoints");
        let trailed = header(&walked[1], "waypoints");
        assert_eq!(banged, ["feat: drop the old importer"]);
        assert_eq!(trailed, ["fix!: tighten the envelope"]);
        assert_eq!(v("1.2.0").cut(&banged).unwrap(), v("1.3.0-pre.1"));
        assert_eq!(v("1.2.0").cut(&trailed).unwrap(), v("2.0.0-pre.1"));
    }

    /// A library right-shifts before `1.0`, as Cargo reads it: a feature is a
    /// patch and a break is a minor.
    #[test]
    fn a_library_right_shifts_before_one() {
        let base = v("0.3.0");
        assert_eq!(
            base.next_library(&["feat(bedrock)!: a break".to_owned()]),
            v("0.4.0")
        );
        assert_eq!(
            base.next_library(&["feat(bedrock): a feature".to_owned()]),
            v("0.3.1")
        );
        assert_eq!(base.next_library(&["fix: a bug".to_owned()]), v("0.3.1"));
        assert_eq!(
            v("1.4.2").next_library(&["feat(outpost): a feature".to_owned()]),
            v("1.5.0")
        );
    }
}
