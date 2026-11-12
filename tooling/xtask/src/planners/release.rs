// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plans the four ref-minting transitions of a release cycle.
//!
//! The arithmetic of each transition is [`Version::transition`]'s; this module
//! supplies what it is applied to (the app's version, or for a cut the last
//! stable tag and the commits since it), shapes the answer as the outputs the
//! workflow reads back, and leaves the version write in the working tree for the
//! pull-request step to land on the protected branch.
//!
//! Only `cut` consults commit history, because `main` is the only ref whose
//! history describes the work honestly. A release branch carries cherry-picks
//! whose messages describe the original change rather than its severity as a
//! backport, so `advance` and `patch` increment mechanically.
//!
//! Nightlies are not planned here: their version is a timestamp rather than a
//! claim about content. See [`super::nightly`].

use crate::{
    planners::test,
    release_groups::{self, GroupFile},
    releases::{
        index,
        landing::Landing,
        version::{APP_MANIFEST, Transition, Version},
    },
    tools::{git, github},
    workspace,
};
use anyhow::{Context as _, Result, bail};
use cargo_metadata::Metadata;
use std::{fs, path::Path, process::Command};

/// The branch a cut's bump lands on.
const MAIN: &str = "main";

/// Records every workspace crate's version, so a version write moves it too.
const LOCKFILE: &str = "Cargo.lock";

/// A transition's pull requests come from here, one branch per target, since a
/// cut opens one against the series and one against `main` in a single run.
const HEAD_PREFIX: &str = "botswain/release-plan/";

/// What the planner decided, and what the workflow reads back as job outputs.
///
/// Serialized field-for-field into `$GITHUB_OUTPUT`, so these field names are
/// the output keys.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct Plan {
    /// The version to write into the manifest on `branch`.
    pub version: String,
    /// The tag to create, always `v{version}`.
    pub tag: String,
    /// The branch the commit and tag land on.
    pub branch: String,
    /// `preview` or `stable` — derived from `version`, never stored.
    pub channel: String,
    /// Where `main` moves to. Empty except on `cut`.
    pub next_main_version: String,
}

impl Plan {
    /// The plan `transition` makes from `from`: the last stable release for a
    /// cut, the branch's current version otherwise. `commits` matter only to
    /// a cut — see [`Version::transition`].
    pub fn new(transition: Transition, from: &Version, commits: &[String]) -> Result<Self> {
        let version = from.transition(transition, commits)?;
        let next_main = match transition {
            Transition::Cut => Some(version.target().next_main()?),
            _ => None,
        };
        Ok(Self {
            tag: version.tag(),
            branch: version.release_branch(),
            channel: version.channel().to_string(),
            next_main_version: next_main.map(|v| v.to_string()).unwrap_or_default(),
            version: version.to_string(),
        })
    }

    /// The message for the version commit this plan's tag points at.
    pub fn commit_message(&self) -> String {
        format!(
            "{RELEASE_PREFIX}{branch} {channel} {version}",
            branch = self.branch,
            channel = self.channel,
            version = self.version,
        )
    }

    /// The pull request that lands this plan's version write on its series.
    pub fn landing(&self, groups: &GroupFile) -> Result<Landing> {
        landing(&self.branch, self.commit_message(), groups)
    }

    /// The pull request that lands `main`'s bump. Only a cut has one.
    pub fn main_landing(&self, groups: &GroupFile) -> Result<Option<Landing>> {
        if self.next_main_version.is_empty() {
            return Ok(None);
        }
        let next = Version::parse(&self.next_main_version)?;
        main_landing(&next, groups).map(Some)
    }
}

fn landing(target: &str, title: String, groups: &GroupFile) -> Result<Landing> {
    Ok(Landing::new(
        format!("{HEAD_PREFIX}{target}"),
        title,
        &written_paths(groups)?,
    ))
}

fn main_landing(next: &Version, groups: &GroupFile) -> Result<Landing> {
    landing(MAIN, next_main_message(next), groups)
}

/// The crates an app version write sets together: the app's group, which the
/// pull-request check holds to one version.
fn app_members(groups: &GroupFile) -> Result<&[String]> {
    let anchor = APP_MANIFEST.trim_end_matches("/Cargo.toml");
    groups
        .members_anchored_at(anchor)
        .with_context(|| format!("no group is anchored at `{anchor}`"))
}

/// What a version write changes: every member's manifest, and the lockfile
/// entries that repeat their versions.
fn written_paths(groups: &GroupFile) -> Result<Vec<String>> {
    Ok(app_members(groups)?
        .iter()
        .map(|member| format!("{member}/Cargo.toml"))
        .chain([LOCKFILE.to_owned()])
        .collect())
}

/// What every commit the planner writes is prefixed with. Release bookkeeping
/// is a conventional `chore`, which `.config/cliff.toml` both accepts and
/// keeps out of the notes.
///
/// Not cosmetic: the release's own `git-cliff` step runs under
/// `require_conventional`, and it runs *after* the tag exists. An
/// unconventional message there mints the tag and then fails the draft, which
/// skips every build — as `v0.1.x preview 0.1.0-pre.1` did.
const RELEASE_PREFIX: &str = "chore(release): ";

/// The message for `main`'s post-cut bump.
pub fn next_main_message(next: &Version) -> String {
    format!("{RELEASE_PREFIX}main now targets {next}")
}

/// Plans `transition` from the checkout: the app's current version, or for
/// a cut the stable tags and the commits since the last one. Refuses a tree
/// the group file does not describe.
pub fn plan(transition: Transition) -> Result<Plan> {
    crate::release_groups::load(&crate::workspace::load()?)?;

    let (from, commits) = match transition {
        Transition::Cut => {
            let tags = stable_tags()?;
            let last = last_stable(&tags);
            let commits = commits_since(last.as_ref())?;
            // Nothing released yet: the first cut is measured from 0.0.0, and
            // `log_range` reads the whole history rather than a missing tag.
            (last.unwrap_or_else(|| Version::new(0, 0, 0)), commits)
        }
        _ => (crate::workspace::version()?.into(), Vec::new()),
    };
    Plan::new(transition, &from, &commits)
}

/// Leaves `plan`'s version write in the working tree on its series, for the
/// pull-request step to commit. A cut opens the series first.
pub fn execute(transition: Transition, plan: &Plan) -> Result<()> {
    if transition == Transition::Cut {
        github::create_branch(&plan.branch, &git::run(&["rev-parse", "HEAD"])?)?;
    }
    git::check_out_latest(&plan.branch)?;
    write_version(&Version::parse(&plan.version)?)
}

/// Leaves `main`'s post-cut bump in the working tree, for the pull-request step
/// to commit.
pub fn bump_main(next: &Version) -> Result<Landing> {
    git::check_out_latest(MAIN)?;
    write_version(next)?;
    main_landing(next, &release_groups::load(&workspace::load()?)?)
}

/// The body of every pull request the planner opens. It becomes the merge
/// commit's body, so it says what the merge commit is; GitHub wraps it at 72
/// columns, so it stays under them.
pub const PULL_REQUEST_BODY: &str =
    "Written by `cargo xtask release-plan`; merges when its checks pass.";

/// What a dry run renders instead of writing, each file by name, for the task
/// to put under one directory: `series.patch` and, for a cut, `main.patch` are
/// the version writes as unified diffs against the trees they would land on;
/// `refs.md` names the branch, the tag, and the pull requests; `notes.md` is
/// git-cliff's rendering for the would-be tag. Everything is read with `git
/// show`, so the checkout is left as it was.
pub fn rehearse(transition: Transition, plan: &Plan) -> Result<Vec<(String, String)>> {
    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let version = Version::parse(&plan.version)?;

    // A cut opens its series from the commit the run is on; the others write
    // on a series that exists, which the checkout may not be on.
    let series_tree = if transition == Transition::Cut {
        "HEAD".to_owned()
    } else {
        remote_or_local(&plan.branch)
    };
    let mut files = vec![(
        "series.patch".to_owned(),
        version_patch(&metadata, &groups, &series_tree, &version)?,
    )];

    let series = plan.landing(&groups)?;
    let main = plan.main_landing(&groups)?;
    if !plan.next_main_version.is_empty() {
        let next = Version::parse(&plan.next_main_version)?;
        files.push((
            "main.patch".to_owned(),
            version_patch(&metadata, &groups, "HEAD", &next)?,
        ));
    }
    files.push((
        "refs.md".to_owned(),
        refs_markdown(transition, plan, &series, main.as_ref()),
    ));
    files.push(("notes.md".to_owned(), unreleased_notes(&plan.tag)?));
    Ok(files)
}

/// The remote's view of `branch` where the checkout has fetched it, else the
/// local branch — and a name `git show` fails on, by name, when neither exists.
fn remote_or_local(branch: &str) -> String {
    let remote = format!("origin/{branch}");
    match git::run(&["rev-parse", "--verify", "--quiet", &remote]) {
        Ok(_) => remote,
        Err(_) => branch.to_owned(),
    }
}

/// The write [`write_version`] would make in the tree at `reference`, as one
/// unified diff. Computed from `git show`, so nothing in the checkout moves.
fn version_patch(
    metadata: &Metadata,
    groups: &GroupFile,
    reference: &str,
    version: &Version,
) -> Result<String> {
    let packages = test::package_directories(metadata);
    let before = git::show(reference, LOCKFILE)?;
    let mut lock = before.clone();
    let mut changes = Vec::new();
    for member in app_members(groups)? {
        let path = format!("{member}/Cargo.toml");
        let manifest = git::show(reference, &path)?;
        let rewritten = set_package_version(&manifest, version)?;
        changes.push((path, manifest, rewritten));

        let (package, _) = packages
            .iter()
            .find(|(_, directory)| directory == member)
            .with_context(|| format!("`{member}` is not a workspace member"))?;
        lock = set_locked_version(&lock, package, version)?;
    }
    changes.push((LOCKFILE.to_owned(), before, lock));
    unified_diff(&changes)
}

/// `changes` — `(path, before, after)` — as `git diff` prints them between two
/// trees, from a scratch directory holding both sides under `a/` and `b/`.
fn unified_diff(changes: &[(String, String, String)]) -> Result<String> {
    let scratch = std::env::temp_dir().join(format!("xtask-release-plan-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    for (path, before, after) in changes {
        for (side, text) in [("a", before), ("b", after)] {
            let file = scratch.join(side).join(path);
            let parent = file.parent().context("a change has a parent directory")?;
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
            write(&file, text)?;
        }
    }
    // `--no-prefix`: the `a/` and `b/` directories are the prefixes. Like
    // `diff`, the command exits 1 when the sides differ.
    let output = Command::new("git")
        .args([
            "diff",
            "--no-index",
            "--no-color",
            "--no-prefix",
            "--",
            "a",
            "b",
        ])
        .current_dir(&scratch)
        .output()
        .context("running `git diff --no-index`")?;
    let _ = fs::remove_dir_all(&scratch);
    match output.status.code() {
        Some(0 | 1) => String::from_utf8(output.stdout).context("the diff is not UTF-8"),
        _ => bail!(
            "`git diff --no-index` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}

/// The refs a dry run would mint and the pull requests it would open.
fn refs_markdown(
    transition: Transition,
    plan: &Plan,
    series: &Landing,
    main: Option<&Landing>,
) -> String {
    let transition = format!("{transition:?}").to_lowercase();
    let branch = if plan.next_main_version.is_empty() {
        format!("`{}`", plan.branch)
    } else {
        format!("`{}`, created from the commit this run is on", plan.branch)
    };
    let mut out = format!(
        "# Dry run: {transition}\n\n\
         - series branch: {branch}\n\
         - tag: `{tag}`, on the merge commit of the series pull request\n\
         - series pull request: {series}\n  body: {PULL_REQUEST_BODY}\n",
        tag = plan.tag,
        series = series.describe(&plan.branch),
    );
    if let Some(main) = main {
        out += &format!(
            "- main moves to `{next}`\n\
             - main pull request: {main}\n  body: {PULL_REQUEST_BODY}\n",
            next = plan.next_main_version,
            main = main.describe(MAIN),
        );
    }
    out
}

/// git-cliff's notes for the tag this plan would mint: everything unreleased,
/// under that name. The real draft renders `--latest` once the tag exists.
fn unreleased_notes(tag: &str) -> Result<String> {
    let output = Command::new("git-cliff")
        .args(["--unreleased", "--tag", tag])
        .output()
        .context("running git-cliff, which the xtask lane installs")?;
    if !output.status.success() {
        bail!(
            "git-cliff failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("the notes are not UTF-8")
}

#[cfg(test)]
mod rehearsal_tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    /// Scenario _A dry cut_, as far as the rendering goes: the patch is the
    /// write `write_version` would make, under the paths it would touch, with
    /// the final newline intact on both sides.
    #[test]
    fn the_patch_is_the_version_write_under_its_paths() {
        let manifest = "[package]\nname = \"waypoints\"\nversion = \"0.1.0-nightly\"\n";
        let rewritten = set_package_version(manifest, &v("0.2.0-pre.1")).unwrap();
        let patch = unified_diff(&[(
            "src-tauri/Cargo.toml".to_owned(),
            manifest.to_owned(),
            rewritten,
        )])
        .unwrap();
        assert!(patch.contains("--- a/src-tauri/Cargo.toml\n"), "{patch}");
        assert!(patch.contains("+++ b/src-tauri/Cargo.toml\n"), "{patch}");
        assert!(patch.contains("-version = \"0.1.0-nightly\"\n"), "{patch}");
        assert!(patch.contains("+version = \"0.2.0-pre.1\"\n"), "{patch}");
        assert!(!patch.contains("No newline"), "{patch}");
    }

    /// Nothing to write renders an empty patch rather than a failure.
    #[test]
    fn an_unchanged_tree_is_an_empty_patch() {
        let text = "[package]\nversion = \"0.1.0\"\n".to_owned();
        let patch = unified_diff(&[("Cargo.toml".to_owned(), text.clone(), text)]).unwrap();
        assert_eq!(patch, "");
    }

    /// The refs a reader audits before the first real cut: branch, tag, both
    /// pull requests and their body, and where `main` moves.
    #[test]
    fn a_dry_cut_names_its_refs() {
        let plan = Plan::new(Transition::Cut, &v("0.1.0"), &["feat: charts".to_owned()]).unwrap();
        let series = Landing::new(
            "botswain/release-plan/v0.2.x".to_owned(),
            plan.commit_message(),
            &["src-tauri/Cargo.toml".to_owned(), LOCKFILE.to_owned()],
        );
        let main = Landing::new(
            "botswain/release-plan/main".to_owned(),
            next_main_message(&v("0.3.0-nightly")),
            &["src-tauri/Cargo.toml".to_owned()],
        );
        let refs = refs_markdown(Transition::Cut, &plan, &series, Some(&main));
        assert!(refs.starts_with("# Dry run: cut\n"), "{refs}");
        assert!(
            refs.contains("`v0.2.x`, created from the commit this run is on"),
            "{refs}"
        );
        assert!(refs.contains("- tag: `v0.2.0-pre.1`"), "{refs}");
        assert!(
            refs.contains("from botswain/release-plan/v0.2.x into v0.2.x"),
            "{refs}"
        );
        assert!(refs.contains("- main moves to `0.3.0-nightly`"), "{refs}");
        assert!(
            refs.contains("from botswain/release-plan/main into main"),
            "{refs}"
        );
        assert_eq!(refs.matches(PULL_REQUEST_BODY).count(), 2, "{refs}");

        let advance = Plan::new(Transition::Advance, &v("0.2.0-pre.1"), &[]).unwrap();
        let refs = refs_markdown(Transition::Advance, &advance, &series, None);
        assert!(refs.contains("- series branch: `v0.2.x`\n"), "{refs}");
        assert!(!refs.contains("main moves"), "{refs}");
    }
}

/// Writes `version` to every member of the app's group, in the tree checked out
/// now: their manifests and their lockfile entries.
fn write_version(version: &Version) -> Result<()> {
    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let packages = test::package_directories(&metadata);
    let root = metadata.workspace_root.as_std_path();

    let lockfile = root.join(LOCKFILE);
    let mut lock = read(&lockfile)?;
    for member in app_members(&groups)? {
        let manifest = root.join(member).join("Cargo.toml");
        write(&manifest, &set_package_version(&read(&manifest)?, version)?)?;

        let (package, _) = packages
            .iter()
            .find(|(_, directory)| directory == member)
            .with_context(|| format!("`{member}` is not a workspace member"))?;
        lock = set_locked_version(&lock, package, version)?;
    }
    write(&lockfile, &lock)
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))
}

fn write(path: &Path, text: &str) -> Result<()> {
    fs::write(path, text).with_context(|| format!("Failed to write {}", path.display()))
}

/// Rewrites the `[package]` version, leaving every other `version =` alone.
///
/// Dependencies carry `version` too, so this walks to the `[package]` table
/// first and edits the first assignment inside it rather than pattern-matching
/// the file. The key keeps its indent and alignment, so the write never also
/// reads as a reformat.
fn set_package_version(manifest: &str, version: &Version) -> Result<String> {
    let mut in_package = false;
    let mut edited = false;

    let out: Vec<String> = manifest
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('[') {
                in_package = trimmed.starts_with("[package]");
            } else if in_package
                && !edited
                && let Some((key, _)) = line.split_once('=')
                && key.trim() == "version"
            {
                edited = true;
                return format!("{key}= \"{version}\"");
            }
            line.to_owned()
        })
        .collect();

    if !edited {
        bail!("no `[package]` version to rewrite");
    }
    Ok(out.join("\n") + "\n")
}

/// Rewrites a workspace crate's `Cargo.lock` entry. A registry crate sharing its
/// name carries a `source` line, and a workspace crate never does.
fn set_locked_version(lock: &str, package: &str, version: &Version) -> Result<String> {
    let name = format!("name = \"{package}\"");
    let mut lines: Vec<String> = lock.lines().map(str::to_owned).collect();

    let entry = (0..lines.len()).find(|&index| {
        lines[index] == name
            && lines
                .get(index + 1)
                .is_some_and(|line| line.starts_with("version = "))
            && !lines
                .get(index + 2)
                .is_some_and(|line| line.starts_with("source = "))
    });
    let Some(entry) = entry else {
        bail!("{LOCKFILE} has no workspace entry for `{package}`");
    };
    lines[entry + 1] = format!("version = \"{version}\"");
    Ok(lines.join("\n") + "\n")
}

/// The newest stable tag, which is what a cut measures its work against.
///
/// Previews and nightlies are excluded: both are views of a release that has
/// not happened yet, so neither marks the end of a cycle.
fn last_stable(tags: &[String]) -> Option<Version> {
    tags.iter()
        .filter_map(|tag| tag.parse::<Version>().ok())
        .filter(Version::is_stable)
        .max()
}

fn stable_tags() -> Result<Vec<String>> {
    Ok(git::run(&["tag", "--list", "v*"])?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// The `git log` range covering one cycle's work.
///
/// Before the first release there is no tag to measure from, and naming one
/// that does not exist makes `git log` fail — so the whole history is the
/// cycle.
fn log_range(since: Option<&Version>) -> String {
    match since {
        Some(version) => format!("{}..HEAD", version.tag()),
        None => "HEAD".to_owned(),
    }
}

/// Conventional-commit subjects for one cycle, oldest first.
///
/// Index commits are left out: they land on `main` between releases, and
/// reading them as work would nudge the target a cut is heading toward.
fn commits_since(since: Option<&Version>) -> Result<Vec<String>> {
    index::product_commits(&log_range(since))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::releases::version::transitions;
    use clap::ValueEnum;
    use strum::IntoEnumIterator as _;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }
    fn msgs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }
    fn plan(transition: Transition, from: &str, commits: &[&str]) -> Plan {
        Plan::new(transition, &v(from), &msgs(commits)).unwrap()
    }

    /// `git-cliff` runs under `require_conventional`, and it runs after the
    /// tag is minted — so an unconventional message here fails a release that
    /// is already tagged, and skips every build behind the draft. Every
    /// message the planner writes must parse as `type(scope): description`,
    /// and cliff must keep it out of the notes.
    #[test]
    fn every_message_the_planner_writes_is_conventional_and_skipped() {
        let messages = [
            plan(Transition::Cut, "0.1.0", &["feat: charts"]).commit_message(),
            plan(Transition::Advance, "0.2.0-pre.1", &[]).commit_message(),
            plan(Transition::Promote, "0.2.0-pre.7", &[]).commit_message(),
            plan(Transition::Patch, "0.2.0", &[]).commit_message(),
            next_main_message(&v("0.3.0-nightly")),
        ];

        for message in &messages {
            assert!(message.starts_with(RELEASE_PREFIX), "{message}");
            let (header, description) = message.split_once(": ").expect(message);
            assert!(!header.contains(' '), "{message}");
            assert!(!description.is_empty(), "{message}");
        }

        // The prefix is only usable because cliff's parsers skip it: a
        // release whose notes listed its own version bump would be noise.
        let cliff = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.config/cliff.toml"),
        )
        .expect("cliff.toml");
        assert!(
            cliff
                .lines()
                .any(|line| line.contains(r"^chore\\(release\\):") && line.contains("skip = true")),
            "cliff.toml no longer skips the planner's commits"
        );
    }

    /// The index commits land on `main` between releases, and the cut reads
    /// that history — so the target has to come out the same with them there
    /// or not, even though each is worded as a feature.
    #[test]
    fn index_commits_do_not_move_a_cut_target() {
        use crate::releases::index::fixture::Repo;

        let repo = Repo::new();
        repo.product("feat: charts");
        repo.tag("v0.1.0");
        repo.product("fix: a bug");
        let without = repo.product_commits("v0.1.0..HEAD");

        repo.index("stable", "v0.1.0");
        repo.index("nightly", "v0.2.0-nightly.20260829");
        let with = repo.product_commits("v0.1.0..HEAD");

        assert_eq!(with, without);
        assert_eq!(v("0.1.0").cut(&with).unwrap(), v("0.1.1-pre.1"));
        assert_eq!(
            v("0.1.0").cut(&with).unwrap(),
            v("0.1.0").cut(&without).unwrap()
        );
    }

    #[test]
    fn a_cut_opens_a_preview_line_and_moves_main() {
        let plan = plan(Transition::Cut, "0.1.0", &["feat: charts", "fix: typo"]);
        assert_eq!(plan.version, "0.2.0-pre.1");
        assert_eq!(plan.tag, "v0.2.0-pre.1");
        assert_eq!(plan.branch, "v0.2.x");
        assert_eq!(plan.channel, "preview");
        assert_eq!(plan.next_main_version, "0.3.0-nightly");

        let patch = plan_for_fix_only();
        assert_eq!(patch.version, "0.1.1-pre.1");
        assert_eq!(patch.branch, "v0.1.x");
        assert_eq!(patch.next_main_version, "0.2.0-nightly");
    }

    fn plan_for_fix_only() -> Plan {
        plan(Transition::Cut, "0.1.0", &["fix: a bug"])
    }

    /// Only a cut moves `main`; the other transitions leave it alone.
    #[test]
    fn only_a_cut_names_a_next_main_version() {
        for (transition, from) in [
            (Transition::Advance, "0.2.0-pre.1"),
            (Transition::Promote, "0.2.0-pre.7"),
            (Transition::Patch, "0.2.0"),
        ] {
            let plan = plan(transition, from, &[]);
            assert_eq!(plan.next_main_version, "", "{transition:?}");
            assert_eq!(plan.branch, "v0.2.x", "{transition:?}");
        }
        assert_eq!(
            plan(Transition::Promote, "0.2.0-pre.7", &[]).channel,
            "stable"
        );
        assert_eq!(plan(Transition::Patch, "0.2.0", &[]).tag, "v0.2.1");
    }

    #[test]
    fn a_plan_refuses_the_wrong_channel() {
        assert!(Plan::new(Transition::Advance, &v("0.2.0"), &[]).is_err());
        assert!(Plan::new(Transition::Promote, &v("0.2.0"), &[]).is_err());
        assert!(Plan::new(Transition::Patch, &v("0.2.0-pre.1"), &[]).is_err());
    }

    #[test]
    fn only_stable_tags_anchor_a_cut() {
        let tags = msgs(&[
            "v0.1.0",
            "v0.2.0-pre.3",
            "v0.2.0-nightly.20260829",
            "v0.1.2",
        ]);
        assert_eq!(last_stable(&tags), Some(v("0.1.2")));
    }

    #[test]
    fn a_repository_with_no_release_yet_cuts_from_zero() {
        assert_eq!(last_stable(&[]), None);
    }

    /// Naming a tag that does not exist makes `git log` fail, so the first
    /// cut in an untagged repository must read the whole history instead.
    #[test]
    fn the_first_cut_reads_the_whole_history() {
        assert_eq!(log_range(None), "HEAD");
        assert_eq!(log_range(Some(&v("0.1.0"))), "v0.1.0..HEAD");
    }

    #[test]
    fn only_the_package_version_is_rewritten() {
        let manifest = "\
[package]
name = \"Waypoints\"
version = \"0.1.0\"
edition = \"2024\"

[dependencies]
domain = { version = \"0.3.1\", path = \"../crates/domain\" }
serde = \"1\"

[build-dependencies]
version = \"ignored\"
";
        let out = set_package_version(manifest, &v("0.2.0-pre.1")).unwrap();
        assert!(out.contains("version = \"0.2.0-pre.1\""));
        // a dependency's own version must survive untouched
        assert!(out.contains("domain = { version = \"0.3.1\""));
        // ...as must a `version` key in some other table
        assert!(out.contains("version = \"ignored\""));
        assert_eq!(out.matches("0.2.0-pre.1").count(), 1);
    }

    #[test]
    fn rewriting_a_manifest_without_a_package_version_is_an_error() {
        assert!(set_package_version("[dependencies]\nserde = \"1\"\n", &v("0.2.0")).is_err());
    }

    /// The bot's commit has to carry the version write and nothing else, and
    /// these manifests are aligned by tombi.
    #[test]
    fn the_rewrite_keeps_the_line_s_indent_and_alignment() {
        let manifest = "[package]\n  name              = \"cartograph\"\n  version           = \"0.1.0\"\n  edition.workspace = true\n";
        assert_eq!(
            set_package_version(manifest, &v("0.2.0-pre.1")).unwrap(),
            "[package]\n  name              = \"cartograph\"\n  version           = \"0.2.0-pre.1\"\n  edition.workspace = true\n"
        );
    }

    /// A crate inheriting its version has nothing of its own to rewrite.
    #[test]
    fn an_inherited_version_is_not_rewritten() {
        let manifest = "[package]\nversion.workspace = true\n";
        assert!(set_package_version(manifest, &v("0.2.0")).is_err());
    }

    fn workspace_groups() -> GroupFile {
        release_groups::load(&workspace::load().unwrap()).unwrap()
    }

    /// The pull-request check holds the whole group to one version, so every
    /// transition has to write all of it in one commit.
    #[test]
    fn every_transition_writes_the_whole_group_and_its_lockfile() {
        let groups = workspace_groups();
        for (transition, from, commits) in [
            (Transition::Cut, "0.1.0", &["feat: charts"][..]),
            (Transition::Advance, "0.2.0-pre.1", &[]),
            (Transition::Promote, "0.2.0-pre.7", &[]),
            (Transition::Patch, "0.2.0", &[]),
        ] {
            let plan = plan(transition, from, commits);
            let landing = plan.landing(&groups).unwrap();
            assert_eq!(
                landing.paths, "src-tauri/Cargo.toml,crates/cartograph/Cargo.toml,Cargo.lock",
                "{transition:?}"
            );
            assert_eq!(
                landing.head,
                format!("botswain/release-plan/{}", plan.branch)
            );
            assert_eq!(landing.title, plan.commit_message());
        }
    }

    /// Two pull requests in one run, so they cannot share a branch: the step
    /// refreshes its branch in place, and the second would overwrite the first.
    #[test]
    fn only_a_cut_lands_on_main_and_from_its_own_branch() {
        let groups = workspace_groups();

        let cut = plan(Transition::Cut, "0.1.0", &["feat: charts"]);
        let series = cut.landing(&groups).unwrap();
        let main = cut
            .main_landing(&groups)
            .unwrap()
            .expect("a cut bumps main");
        assert_ne!(series.head, main.head);
        assert_eq!(main.head, "botswain/release-plan/main");
        assert_eq!(main.title, next_main_message(&v("0.3.0-nightly")));
        assert_eq!(main.paths, series.paths);

        let advance = plan(Transition::Advance, "0.2.0-pre.1", &[]);
        assert_eq!(advance.main_landing(&groups).unwrap(), None);
    }

    const LOCK: &str = "\
[[package]]
name = \"cartograph\"
version = \"0.1.0\"
dependencies = [
 \"domain\",
]

[[package]]
name = \"serde\"
version = \"0.1.0\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
";

    #[test]
    fn a_lock_entry_moves_alone() {
        let out = set_locked_version(LOCK, "cartograph", &v("0.2.0-pre.1")).unwrap();
        assert_eq!(out, LOCK.replacen("0.1.0", "0.2.0-pre.1", 1));
    }

    /// A workspace crate never has a `source`, so one that does is someone
    /// else's crate of the same name.
    #[test]
    fn a_registry_crate_is_never_a_workspace_entry() {
        assert!(set_locked_version(LOCK, "serde", &v("9.9.9")).is_err());
        assert!(set_locked_version(LOCK, "absent", &v("9.9.9")).is_err());
    }

    #[test]
    fn this_lockfile_records_every_member_of_the_app_s_group() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let lock = fs::read_to_string(root.join(LOCKFILE)).unwrap();
        for package in ["Waypoints", "cartograph"] {
            let out = set_locked_version(&lock, package, &v("99.0.0")).unwrap();
            let changed = lock
                .lines()
                .zip(out.lines())
                .filter(|(before, after)| before != after)
                .count();
            assert_eq!(changed, 1, "{package}");
        }
    }

    /// The manifest the tag names must be the one the tag points at.
    #[test]
    fn the_rewritten_version_is_the_tagged_one() {
        let plan = plan(Transition::Patch, "0.2.0", &[]);
        let manifest = "[package]\nversion = \"0.2.0\"\n";
        let out = set_package_version(manifest, &Version::parse(&plan.version).unwrap()).unwrap();
        assert!(out.contains("version = \"0.2.1\""));
        assert_eq!(plan.tag, "v0.2.1");
    }

    /// The dispatch input offers exactly the values clap accepts, so a run
    /// cannot ask for a transition the CLI cannot parse.
    #[test]
    fn every_offered_transition_is_a_real_cli_value() {
        let offered = transitions();
        assert_eq!(offered.len(), Transition::iter().count());

        let accepted: Vec<String> = Transition::value_variants()
            .iter()
            .filter_map(ValueEnum::to_possible_value)
            .map(|value| value.get_name().to_owned())
            .collect();

        for transition in &offered {
            assert!(
                accepted.contains(transition),
                "the workflow offers `{transition}`, which `release-plan` cannot parse; \
                 clap knows {accepted:?}"
            );
        }
    }

    /// The staircase must hold: nightly < preview < stable, then the next cut
    /// starts above the release it followed.
    #[test]
    fn channels_stay_ordered_across_a_cycle() {
        let ordered = [
            v("0.2.0-nightly.20260829"),
            v("0.2.0-pre.1"),
            v("0.2.0-pre.2"),
            v("0.2.0"),
            v("0.2.1"),
            v("0.3.0-pre.1"),
        ];
        let mut sorted = ordered.to_vec();
        sorted.sort();
        assert_eq!(sorted, ordered, "channel ordering is not monotonic");
    }
}
