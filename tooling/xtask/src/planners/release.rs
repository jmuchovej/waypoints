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
    release_groups::{self, GROUP_FILE, GroupFile},
    releases::{
        history,
        landing::Landing,
        version::{APP_MANIFEST, Transition, Version},
    },
    tools::{git, github},
    workspace,
};
use anyhow::{Context as _, Result, bail};
use cargo_metadata::Metadata;
use std::{collections::BTreeSet, fs, path::Path, process::Command};

/// What a cut is asked for beyond its transition.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CutOptions {
    /// Target the app's next major. A major is a product decision no commit
    /// can prove, so it is never computed, only asked for.
    pub major: bool,
    /// Cut one library group alone: its tag on `main`, no series, nothing
    /// else moved.
    pub group: Option<String>,
    /// The least a group-scoped cut bumps.
    pub floor: Option<Floor>,
}

/// The least a group-scoped cut bumps: raises what the commits declare,
/// never lowers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Floor {
    Major,
    Minor,
    Patch,
}

impl Floor {
    /// The lowest version above `current` this floor admits.
    fn applied(self, current: &Version) -> Version {
        match self {
            Floor::Major => Version::new(current.major + 1, 0, 0),
            Floor::Minor => Version::new(current.major, current.minor + 1, 0),
            Floor::Patch => Version::new(current.major, current.minor, current.patch + 1),
        }
    }
}

/// One library group's part of a cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupPlan {
    pub group: String,
    /// The anchor manifest's version: what the group is now.
    pub current: String,
    /// What it becomes, or none when no counted commit landed since its tag.
    pub next: Option<String>,
    /// The tag a cut mints for it, when `next` is some.
    pub tag: Option<String>,
    /// The tag it was last minted at, or none before its first cut.
    pub last_tag: Option<String>,
    /// The commits that counted for it since that tag.
    pub counted: usize,
}

impl GroupPlan {
    /// Decides `group` from its tag `template`, its `current` version, the
    /// remote's `tags`, and the `commits` since its last tag, raised to
    /// `floor` where one is named. A group with no counted commit stays where
    /// it is; before its first tag, its anchor manifest is its version and
    /// the whole history its cycle, as the app's first cut is measured.
    pub fn decide(
        group: &str,
        template: &str,
        current: &Version,
        tags: &[String],
        commits: &[history::Commit],
        groups: &GroupFile,
        floor: Option<Floor>,
    ) -> Result<Self> {
        let prefix = tag_prefix(template)?;
        let last = latest_group_tag(prefix, tags);
        let headers: Vec<String> = commits
            .iter()
            .filter_map(|commit| commit.header_for(group, groups))
            .collect();
        let counted = headers.len();
        let next = (counted > 0).then(|| {
            let bumped = current.next_library(&headers);
            match floor {
                Some(floor) => bumped.max(floor.applied(current)),
                None => bumped,
            }
        });
        Ok(Self {
            group: group.to_owned(),
            current: current.to_string(),
            tag: next
                .as_ref()
                .map(|next| template.replace("{version}", &next.to_string())),
            next: next.map(|next| next.to_string()),
            last_tag: last.map(|last| format!("{prefix}{last}")),
            counted,
        })
    }

    /// Whether a cut mints a tag for this group.
    pub fn mints(&self) -> bool {
        self.next.is_some()
    }

    /// One line a reader audits before a cut.
    pub fn describe(&self) -> String {
        let since = match &self.last_tag {
            Some(tag) => format!("since `{tag}`"),
            None => "over the whole history, with no tag yet".to_owned(),
        };
        match (&self.next, &self.tag) {
            (Some(next), Some(tag)) => format!(
                "`{group}`: `{current}` → `{next}`, tag `{tag}` on the merge commit of the main pull request ({counted} commits {since})",
                group = self.group,
                current = self.current,
                counted = self.counted,
            ),
            _ => format!(
                "`{group}`: `{current}`, unchanged (no counted commit {since})",
                group = self.group,
                current = self.current,
            ),
        }
    }
}

/// The part of a group's tag template before its version.
fn tag_prefix(template: &str) -> Result<&str> {
    template
        .split_once("{version}")
        .map(|(prefix, _)| prefix)
        .filter(|prefix| !prefix.is_empty())
        .with_context(|| format!("tag template `{template}` has no prefix before `{{version}}`"))
}

/// The newest of a group's tags among `tags`, by version.
fn latest_group_tag(prefix: &str, tags: &[String]) -> Option<Version> {
    tags.iter()
        .filter_map(|tag| tag.strip_prefix(prefix))
        .filter_map(|rest| Version::parse(rest).ok())
        .max()
}

/// A group's cycle: since its last tag, or the whole history before its
/// first — the rule [`log_range`] applies to the app.
fn group_range(prefix: &str, last: Option<&Version>) -> String {
    match last {
        Some(last) => format!("{prefix}{last}..HEAD"),
        None => "HEAD".to_owned(),
    }
}

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
    /// The version to write into the manifest on `branch`. Empty on a
    /// group-scoped cut, which moves no app version.
    pub version: String,
    /// The tag to create, always `v{version}`; empty on a group-scoped cut.
    pub tag: String,
    /// The branch the commit and tag land on; empty on a group-scoped cut.
    pub branch: String,
    /// `preview` or `stable` — derived from `version`, never stored.
    pub channel: String,
    /// Where `main` moves to. Empty except on `cut`.
    pub next_main_version: String,
    /// The library tags a cut mints at the main pull request's merge commit,
    /// comma-separated: `bedrock-v0.2.0,domain-v0.1.1`. Empty when no
    /// library moves.
    pub group_tags: String,
    /// Every library group's part of the cut, minting or not. Not an output:
    /// the workflow reads `group_tags`.
    #[serde(skip)]
    pub groups: Vec<GroupPlan>,
}

impl Plan {
    /// The plan `transition` makes from `from`: the last stable release for a
    /// cut, the branch's current version otherwise. `commits` matter only to
    /// a cut — see [`Version::transition`].
    pub fn new(transition: Transition, from: &Version, commits: &[String]) -> Result<Self> {
        Self::from_version(transition, from.transition(transition, commits)?)
    }

    /// The plan that mints `version` by `transition`, once the version is
    /// decided — by [`Version::transition`], or for a cut by [`cut_target`].
    pub fn from_version(transition: Transition, version: Version) -> Result<Self> {
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
            group_tags: String::new(),
            groups: Vec::new(),
        })
    }

    /// A plan minting one library alone: no app version, no series, one tag
    /// on `main`.
    pub fn library(group: GroupPlan) -> Self {
        Self {
            version: String::new(),
            tag: String::new(),
            branch: String::new(),
            channel: String::new(),
            next_main_version: String::new(),
            group_tags: String::new(),
            groups: Vec::new(),
        }
        .with_groups(vec![group])
    }

    /// The same plan, carrying every group's part of the cut.
    pub fn with_groups(mut self, groups: Vec<GroupPlan>) -> Self {
        self.group_tags = groups
            .iter()
            .filter_map(|group| group.tag.clone())
            .collect::<Vec<_>>()
            .join(",");
        self.groups = groups;
        self
    }

    /// Whether this cut mints a library alone, with no app version behind it.
    pub fn is_group_only(&self) -> bool {
        self.tag.is_empty()
    }

    /// The groups this plan mints.
    pub fn minted(&self) -> impl Iterator<Item = &GroupPlan> {
        self.groups.iter().filter(|group| group.mints())
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
    /// None on a group-scoped cut, which opens no series.
    pub fn landing(&self, groups: &GroupFile) -> Result<Option<Landing>> {
        if self.is_group_only() {
            return Ok(None);
        }
        landing(&self.branch, self.commit_message(), groups).map(Some)
    }

    /// The pull request that lands the `main`-side writes: the app's bump,
    /// each minted group's manifests, or both. None when nothing lands there.
    pub fn main_landing(&self, groups: &GroupFile) -> Result<Option<Landing>> {
        let minted: Vec<&GroupPlan> = self.minted().collect();
        if self.next_main_version.is_empty() && minted.is_empty() {
            return Ok(None);
        }

        let mut paths = Vec::new();
        if !self.next_main_version.is_empty() {
            paths.extend(
                app_members(groups)?
                    .iter()
                    .map(|member| format!("{member}/Cargo.toml")),
            );
        }
        for group in &minted {
            paths.extend(
                members_of(groups, &group.group)?
                    .iter()
                    .map(|member| format!("{member}/Cargo.toml")),
            );
        }
        paths.push(LOCKFILE.to_owned());

        Ok(Some(Landing::new(
            format!("{HEAD_PREFIX}{MAIN}"),
            self.main_title(&minted)?,
            &paths,
        )))
    }

    /// The main-side subject: the app's bump alone, as before; one library
    /// alone, under the subject reserved to the bot; or a summary of the cut,
    /// since its writes touch more than one group, with one header per group
    /// in its body — see [`Plan::summary_headers`].
    fn main_title(&self, minted: &[&GroupPlan]) -> Result<String> {
        if minted.is_empty() {
            return Ok(next_main_message(&Version::parse(&self.next_main_version)?));
        }
        if self.is_group_only()
            && let [only] = minted
        {
            return Ok(format!(
                "{RELEASE_PREFIX}{} {}",
                only.group,
                only.next.as_deref().unwrap_or_default()
            ));
        }
        Ok(format!("summary: cut {}", self.version))
    }

    /// The headers a summary's body carries, one per group it writes, scoped
    /// by the group so the pull-request check reads each.
    pub fn summary_headers(&self, groups: &GroupFile) -> Result<Vec<String>> {
        let mut headers: Vec<String> = self
            .minted()
            .map(|group| {
                format!(
                    "chore({}): {}",
                    group.group,
                    group.next.as_deref().unwrap_or_default()
                )
            })
            .collect();
        if !self.next_main_version.is_empty() {
            headers.push(format!(
                "chore({}): main now targets {}",
                app_group(groups)?,
                self.next_main_version
            ));
        }
        Ok(headers)
    }
}

/// The crates a group's version write sets together.
fn members_of<'a>(groups: &'a GroupFile, name: &str) -> Result<&'a [String]> {
    groups
        .group
        .get(name)
        .map(|group| group.members.as_slice())
        .with_context(|| format!("no group named `{name}`"))
}

fn landing(target: &str, title: String, groups: &GroupFile) -> Result<Landing> {
    Ok(Landing::new(
        format!("{HEAD_PREFIX}{target}"),
        title,
        &written_paths(groups)?,
    ))
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
pub fn plan(transition: Transition, options: CutOptions) -> Result<Plan> {
    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let root = metadata.workspace_root.as_std_path();

    if transition != Transition::Cut {
        let version = Version::from(workspace::version()?).transition(transition, &[])?;
        return Plan::from_version(transition, version);
    }

    let ignored = history::ignored_revisions(root, &groups)?;
    let app = app_group(&groups)?;

    // One library alone: its tag on `main`, nothing else moved, and a refusal
    // where nothing counted since its last tag.
    if let Some(name) = &options.group {
        if name == app {
            bail!("`{name}` is the app; a cut moves it with its series, never alone");
        }
        if !groups.group.contains_key(name) {
            bail!("no group named `{name}` in {GROUP_FILE}");
        }
        let only = group_plan(name, &groups, &metadata, &ignored, options.floor)?;
        if !only.mints() {
            bail!(
                "nothing to cut for `{name}`: no counted commit {}",
                match &only.last_tag {
                    Some(tag) => format!("since `{tag}`"),
                    None => "in its history".to_owned(),
                }
            );
        }
        return Ok(Plan::library(only));
    }

    let planned = groups
        .group
        .keys()
        .filter(|name| name.as_str() != app)
        .map(|name| group_plan(name, &groups, &metadata, &ignored, None))
        .collect::<Result<Vec<_>>>()?;

    let last = last_stable(&stable_tags()?);
    let commits = commits_since(last.as_ref(), &groups, root)?;
    // Nothing released yet: the first cut is measured from 0.0.0, and
    // `log_range` reads the whole history rather than a missing tag.
    let last = last.unwrap_or_else(|| Version::new(0, 0, 0));
    let version = cut_target(&last, &commits, &groups, options.major)?;
    Ok(Plan::from_version(Transition::Cut, version)?.with_groups(planned))
}

/// One library group's part of a cut, from the remote's tags and the commits
/// since its last one.
fn group_plan(
    name: &str,
    groups: &GroupFile,
    metadata: &Metadata,
    ignored: &BTreeSet<String>,
    floor: Option<Floor>,
) -> Result<GroupPlan> {
    let group = &groups.group[name];
    let prefix = tag_prefix(&group.tag)?;
    let current = anchor_version(metadata, &group.anchor)?;
    let tags = git::remote_tags_like(&format!("{prefix}*"))?;
    let last = latest_group_tag(prefix, &tags);
    let commits = history::walk(&group_range(prefix, last.as_ref()), groups, ignored)?;
    GroupPlan::decide(name, &group.tag, &current, &tags, &commits, groups, floor)
}

/// The version the manifest at `anchor` declares.
fn anchor_version(metadata: &Metadata, anchor: &str) -> Result<Version> {
    let root = metadata.workspace_root.as_std_path();
    metadata
        .workspace_packages()
        .iter()
        .find(|package| {
            package
                .manifest_path
                .as_std_path()
                .parent()
                .and_then(|dir| dir.strip_prefix(root).ok())
                .is_some_and(|dir| dir.to_string_lossy() == anchor)
        })
        .map(|package| Version::from(package.version.clone()))
        .with_context(|| format!("`{anchor}` is not a workspace member"))
}

/// The app's target for a cut from `last` over `commits`.
///
/// A major is cut only when asked for: it is a product decision no commit
/// can prove. A `Covenant: break` since the last stable is the floor, not the
/// trigger — without `--major` the cut refuses and names the commits, since a
/// silent minor and an unreviewed major are both wrong. The app's headers
/// are read as [`history::Commit::header_for`] reads them, so a `!` never
/// reaches the bumper and the trailer does.
pub fn cut_target(
    last: &Version,
    commits: &[history::Commit],
    groups: &GroupFile,
    major: bool,
) -> Result<Version> {
    let app = app_group(groups)?;
    if major {
        return last.cut_major();
    }

    let breaking: Vec<String> = commits
        .iter()
        .filter(|commit| commit.covenant_break && commit.counts_for(app))
        .map(|commit| {
            format!(
                "{} {}",
                &commit.sha[..commit.sha.len().min(9)],
                commit.subject
            )
        })
        .collect();
    if !breaking.is_empty() {
        bail!(
            "a covenant break landed since {last}:\n  {}\na major is a product decision: pass `--major` to cut {next}, or hold the cut",
            breaking.join("\n  "),
            next = last.cut_major()?,
        );
    }

    let headers: Vec<String> = commits
        .iter()
        .filter_map(|commit| commit.header_for(app, groups))
        .collect();
    last.cut(&headers)
}

/// The group the app manifest anchors: the one whose version a cut moves
/// with the series.
fn app_group(groups: &GroupFile) -> Result<&str> {
    let anchor = APP_MANIFEST.trim_end_matches("/Cargo.toml");
    groups
        .group
        .iter()
        .find(|(_, group)| group.anchor == anchor)
        .map(|(name, _)| name.as_str())
        .with_context(|| format!("no group is anchored at `{anchor}`"))
}

/// Leaves `plan`'s writes in the working tree for the pull-request step to
/// commit: on a cut, the `main`-side manifests — each minted group's and the
/// app's bump — on `main`; otherwise the app's version on its series. A cut's
/// series is opened later, from the merge commit those writes land as
/// ([`open_series`]).
pub fn execute(transition: Transition, plan: &Plan) -> Result<()> {
    let groups = release_groups::load(&workspace::load()?)?;

    if transition == Transition::Cut {
        git::check_out_latest(MAIN)?;
        for group in plan.minted() {
            let next = Version::parse(group.next.as_deref().unwrap_or_default())?;
            write_members(members_of(&groups, &group.group)?, &next)?;
        }
        if !plan.next_main_version.is_empty() {
            let next = Version::parse(&plan.next_main_version)?;
            write_members(app_members(&groups)?, &next)?;
        }
        return Ok(());
    }

    git::check_out_latest(&plan.branch)?;
    write_members(app_members(&groups)?, &Version::parse(&plan.version)?)
}

/// Opens a cut's series at `commit` — the merge commit of its main pull
/// request, which its first-parent history contains — and leaves the app's
/// `version` written on it for the pull-request step, which reads the
/// returned landing.
pub fn open_series(version: &Version, commit: &str) -> Result<Landing> {
    let plan = Plan::from_version(Transition::Cut, version.clone())?;
    github::create_branch(&plan.branch, commit)?;
    git::check_out_latest(&plan.branch)?;
    let groups = release_groups::load(&workspace::load()?)?;
    write_members(app_members(&groups)?, version)?;
    plan.landing(&groups)?.context("a cut opens a series")
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
    let mut files = Vec::new();

    // A cut opens its series from the commit the run is on; the others write
    // on a series that exists, which the checkout may not be on. A
    // group-scoped cut opens none.
    let series = plan.landing(&groups)?;
    if series.is_some() {
        let series_tree = if transition == Transition::Cut {
            "HEAD".to_owned()
        } else {
            remote_or_local(&plan.branch)
        };
        let writes = member_writes(app_members(&groups)?, &Version::parse(&plan.version)?);
        files.push((
            "series.patch".to_owned(),
            manifests_patch(&metadata, &series_tree, &writes)?,
        ));
    }

    // The main-side commit: the app's bump, each minted group's manifests, or
    // both.
    let main = plan.main_landing(&groups)?;
    if main.is_some() {
        let mut writes = Vec::new();
        if !plan.next_main_version.is_empty() {
            let next = Version::parse(&plan.next_main_version)?;
            writes.extend(member_writes(app_members(&groups)?, &next));
        }
        for group in plan.minted() {
            let next = Version::parse(group.next.as_deref().unwrap_or_default())?;
            writes.extend(member_writes(members_of(&groups, &group.group)?, &next));
        }
        files.push((
            "main.patch".to_owned(),
            manifests_patch(&metadata, "HEAD", &writes)?,
        ));
    }

    files.push((
        "refs.md".to_owned(),
        refs_markdown(transition, plan, series.as_ref(), main.as_ref(), &groups)?,
    ));
    // A library tag is not a release, and draws no notes.
    if !plan.is_group_only() {
        files.push(("notes.md".to_owned(), unreleased_notes(&plan.tag)?));
    }
    Ok(files)
}

/// `version` for each of `members`: one version write's worth of manifests.
fn member_writes(members: &[String], version: &Version) -> Vec<(String, Version)> {
    members
        .iter()
        .map(|member| (member.clone(), version.clone()))
        .collect()
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

/// The `writes` — `(member, version)` — as the tree at `reference` would take
/// them, as one unified diff: each member's manifest and the lockfile entries
/// that repeat their versions. Computed from `git show`, so nothing in the
/// checkout moves.
fn manifests_patch(
    metadata: &Metadata,
    reference: &str,
    writes: &[(String, Version)],
) -> Result<String> {
    let packages = test::package_directories(metadata);
    let before = git::show(reference, LOCKFILE)?;
    let mut lock = before.clone();
    let mut changes = Vec::new();
    for (member, version) in writes {
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

/// The refs a dry run would mint and the pull requests it would open: the
/// series and its tag, the main-side commit with its summary headers, and
/// every library group's part, minting or not.
fn refs_markdown(
    transition: Transition,
    plan: &Plan,
    series: Option<&Landing>,
    main: Option<&Landing>,
    groups: &GroupFile,
) -> Result<String> {
    let transition = format!("{transition:?}").to_lowercase();
    let mut out = format!("# Dry run: {transition}\n\n");
    if let Some(series) = series {
        let branch = if plan.next_main_version.is_empty() {
            format!("`{}`", plan.branch)
        } else {
            format!("`{}`, created from the commit this run is on", plan.branch)
        };
        out += &format!(
            "- series branch: {branch}\n\
             - tag: `{tag}`, on the merge commit of the series pull request\n\
             - series pull request: {series}\n  body: {PULL_REQUEST_BODY}\n",
            tag = plan.tag,
            series = series.describe(&plan.branch),
        );
    }
    if let Some(main) = main {
        if !plan.next_main_version.is_empty() {
            out += &format!("- main moves to `{}`\n", plan.next_main_version);
        }
        out += &format!(
            "- main pull request: {}\n  body: {PULL_REQUEST_BODY}\n",
            main.describe(MAIN)
        );
        if main.title.starts_with("summary:") {
            for header in plan.summary_headers(groups)? {
                out += &format!("    {header}\n");
            }
        }
    }
    if !plan.groups.is_empty() {
        out += "- groups:\n";
        for group in &plan.groups {
            out += &format!("  - {}\n", group.describe());
        }
    }
    Ok(out)
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

    fn groups() -> GroupFile {
        release_groups::load(&workspace::load().unwrap()).unwrap()
    }

    fn group_plan(group: &str, current: &str, next: Option<&str>, counted: usize) -> GroupPlan {
        GroupPlan {
            group: group.to_owned(),
            current: current.to_owned(),
            next: next.map(str::to_owned),
            tag: next.map(|next| format!("{group}-v{next}")),
            last_tag: Some(format!("{group}-v{current}")),
            counted,
        }
    }

    /// The refs a reader audits before the first real cut: branch, tag, both
    /// pull requests and their body, and where `main` moves.
    #[test]
    fn a_dry_cut_names_its_refs() {
        let groups = groups();
        let plan = Plan::new(Transition::Cut, &v("0.1.0"), &["feat: charts".to_owned()]).unwrap();
        let series = plan.landing(&groups).unwrap().unwrap();
        let main = plan.main_landing(&groups).unwrap().unwrap();
        let refs =
            refs_markdown(Transition::Cut, &plan, Some(&series), Some(&main), &groups).unwrap();
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
        assert!(!refs.contains("- groups:"), "{refs}");

        let advance = Plan::new(Transition::Advance, &v("0.2.0-pre.1"), &[]).unwrap();
        let refs =
            refs_markdown(Transition::Advance, &advance, Some(&series), None, &groups).unwrap();
        assert!(refs.contains("- series branch: `v0.2.x`\n"), "{refs}");
        assert!(!refs.contains("main moves"), "{refs}");
    }

    /// Scenario _A dry cut names every group it would mint_: the minted
    /// group's tag and write beside the app's, the unchanged group named as
    /// unchanged, and the summary's headers under the main pull request.
    #[test]
    fn a_dry_cut_names_every_group_it_would_mint() {
        let groups = groups();
        let plan = Plan::new(Transition::Cut, &v("0.1.0"), &["feat: charts".to_owned()])
            .unwrap()
            .with_groups(vec![
                group_plan("bedrock", "0.3.0", Some("0.3.1"), 2),
                group_plan("outpost", "0.1.0", None, 0),
            ]);
        let series = plan.landing(&groups).unwrap().unwrap();
        let main = plan.main_landing(&groups).unwrap().unwrap();
        let refs =
            refs_markdown(Transition::Cut, &plan, Some(&series), Some(&main), &groups).unwrap();

        assert!(refs.contains("summary: cut 0.2.0-pre.1"), "{refs}");
        assert!(refs.contains("    chore(bedrock): 0.3.1\n"), "{refs}");
        assert!(
            refs.contains("    chore(waypoints): main now targets 0.3.0-nightly\n"),
            "{refs}"
        );
        assert!(
            refs.contains(
                "  - `bedrock`: `0.3.0` → `0.3.1`, tag `bedrock-v0.3.1` on the merge commit of the main pull request (2 commits since `bedrock-v0.3.0`)\n"
            ),
            "{refs}"
        );
        assert!(
            refs.contains(
                "  - `outpost`: `0.1.0`, unchanged (no counted commit since `outpost-v0.1.0`)\n"
            ),
            "{refs}"
        );
        assert!(
            main.paths.contains("crates/bedrock/Cargo.toml"),
            "{}",
            main.paths
        );
        assert!(
            !main.paths.contains("crates/outpost/Cargo.toml"),
            "{}",
            main.paths
        );
    }
}

/// Writes `version` to each of `members`, in the tree checked out now: their
/// manifests and their lockfile entries.
fn write_members(members: &[String], version: &Version) -> Result<()> {
    let metadata = workspace::load()?;
    let packages = test::package_directories(&metadata);
    let root = metadata.workspace_root.as_std_path();

    let lockfile = root.join(LOCKFILE);
    let mut lock = read(&lockfile)?;
    for member in members {
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

/// The published tags, from the remote: a cut measures its work from the last
/// release that exists, not the last one this checkout happens to hold.
fn stable_tags() -> Result<Vec<String>> {
    git::remote_tags()
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

/// One cycle's first-parent commits, attributed, oldest first.
///
/// Bookkeeping counts for no group: index entries land on `main` between
/// releases, and reading them as work would nudge the target a cut is
/// heading toward.
fn commits_since(
    since: Option<&Version>,
    groups: &GroupFile,
    root: &Path,
) -> Result<Vec<history::Commit>> {
    let ignored = history::ignored_revisions(root, groups)?;
    history::walk(&log_range(since), groups, &ignored)
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

    /// A commit on `path`, counting for `groups`, with or without the trailer.
    fn commit_on(
        path: &str,
        groups: &[&str],
        subject: &str,
        covenant_break: bool,
    ) -> history::Commit {
        history::Commit {
            sha: "abc1234def5678".to_owned(),
            subject: subject.to_owned(),
            body: if covenant_break {
                "Covenant: break".to_owned()
            } else {
                String::new()
            },
            covenant_break,
            paths: vec![path.to_owned()],
            groups: groups.iter().map(|group| (*group).to_owned()).collect(),
        }
    }

    /// A commit on the app's own paths, with or without the trailer.
    fn app_commit(subject: &str, covenant_break: bool) -> history::Commit {
        commit_on(
            "src-tauri/src/main.rs",
            &["waypoints"],
            subject,
            covenant_break,
        )
    }

    /// A commit on a library's own paths, which the app counts too.
    fn library_commit(group: &str, subject: &str) -> history::Commit {
        commit_on(
            &format!("crates/{group}/src/lib.rs"),
            &[group, "waypoints"],
            subject,
            false,
        )
    }

    /// Scenario _Break marker without the trailer after v1_: `!` is not the
    /// app's signal, so a feature is a minor even on `1.x`.
    #[test]
    fn a_bang_without_the_trailer_cuts_a_minor() {
        let target = cut_target(
            &v("1.2.0"),
            &[app_commit("feat!: drop the old importer", false)],
            &workspace_groups(),
            false,
        )
        .unwrap();
        assert_eq!(target, v("1.3.0-pre.1"));
    }

    /// Scenario _The trailer after v1_: the trailer refuses a cut that did not
    /// ask for a major, naming the commit, and `--major` cuts one.
    #[test]
    fn the_trailer_refuses_a_cut_without_major_and_cuts_one_with_it() {
        let groups = workspace_groups();
        let commits = [app_commit("fix: tighten the envelope", true)];

        let error = cut_target(&v("1.2.0"), &commits, &groups, false).unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains("abc1234de fix: tighten the envelope"),
            "{text}"
        );
        assert!(
            text.contains("--major") && text.contains("2.0.0-pre.1"),
            "{text}"
        );

        let target = cut_target(&v("1.2.0"), &commits, &groups, true).unwrap();
        assert_eq!(target, v("2.0.0-pre.1"));
    }

    /// Scenario _Product major without the trailer_: the flag alone is enough.
    #[test]
    fn a_major_is_cut_when_asked_for_without_a_trailer() {
        let target = cut_target(
            &v("1.2.0"),
            &[app_commit("fix: a bug", false)],
            &workspace_groups(),
            true,
        )
        .unwrap();
        assert_eq!(target, v("2.0.0-pre.1"));
        assert_eq!(v("0.1.0").cut_major().unwrap(), v("1.0.0-pre.1"));
    }

    fn decide(
        group: &str,
        current: &str,
        tags: &[&str],
        commits: &[history::Commit],
        floor: Option<Floor>,
    ) -> GroupPlan {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_owned()).collect();
        GroupPlan::decide(
            group,
            &format!("{group}-v{{version}}"),
            &v(current),
            &tags,
            commits,
            &workspace_groups(),
            floor,
        )
        .unwrap()
    }

    /// Scenario _Cut with library work_: the group moves, its tag is named,
    /// and the main-side commit becomes a summary carrying its manifests.
    #[test]
    fn a_group_with_counted_commits_moves_and_is_tagged() {
        let groups = workspace_groups();
        let bedrock = decide(
            "bedrock",
            "0.3.0",
            &["bedrock-v0.3.0", "v0.1.0"],
            &[library_commit("bedrock", "feat(bedrock): a thing")],
            None,
        );
        assert_eq!(bedrock.next.as_deref(), Some("0.3.1"));
        assert_eq!(bedrock.tag.as_deref(), Some("bedrock-v0.3.1"));
        assert_eq!(bedrock.last_tag.as_deref(), Some("bedrock-v0.3.0"));
        assert_eq!(bedrock.counted, 1);

        let plan = Plan::from_version(Transition::Cut, v("0.2.0-pre.1"))
            .unwrap()
            .with_groups(vec![bedrock]);
        assert_eq!(plan.group_tags, "bedrock-v0.3.1");
        let main = plan.main_landing(&groups).unwrap().unwrap();
        assert_eq!(main.title, "summary: cut 0.2.0-pre.1");
        for path in [
            "crates/bedrock/Cargo.toml",
            "crates/bedrock-core/Cargo.toml",
            "src-tauri/Cargo.toml",
            "Cargo.lock",
        ] {
            assert!(main.paths.contains(path), "{}", main.paths);
        }
        assert_eq!(
            plan.summary_headers(&groups).unwrap(),
            [
                "chore(bedrock): 0.3.1",
                "chore(waypoints): main now targets 0.3.0-nightly",
            ]
        );
    }

    /// Scenario _Cut without library work_: nothing moves and no tag is
    /// named, whatever other groups did.
    #[test]
    fn a_group_without_counted_commits_stays() {
        let outpost = decide(
            "outpost",
            "0.1.0",
            &["outpost-v0.1.0"],
            &[library_commit("bedrock", "feat(bedrock): elsewhere")],
            None,
        );
        assert!(!outpost.mints());
        assert_eq!(outpost.tag, None);
        assert_eq!(outpost.counted, 0);

        let plan = Plan::from_version(Transition::Cut, v("0.2.0-pre.1"))
            .unwrap()
            .with_groups(vec![outpost]);
        assert_eq!(plan.group_tags, "");
        let main = plan.main_landing(&workspace_groups()).unwrap().unwrap();
        assert_eq!(main.title, next_main_message(&v("0.3.0-nightly")));
        assert!(!main.paths.contains("crates/outpost"), "{}", main.paths);
    }

    /// Scenario _First cut of a group with no tag_: the anchor manifest is the
    /// version and the whole history the cycle.
    #[test]
    fn a_group_with_no_tag_is_measured_over_its_whole_history() {
        assert_eq!(group_range("domain-v", None), "HEAD");
        assert_eq!(
            group_range("domain-v", Some(&v("0.1.1"))),
            "domain-v0.1.1..HEAD"
        );
        let domain = decide(
            "domain",
            "0.1.0",
            &["v0.1.0", "bedrock-v0.3.0"],
            &[library_commit("domain", "feat(domain): a thing")],
            None,
        );
        assert_eq!(domain.last_tag, None);
        assert_eq!(domain.next.as_deref(), Some("0.1.1"));
        assert_eq!(domain.tag.as_deref(), Some("domain-v0.1.1"));
    }

    /// Scenario _Group-only cut_: one tag on `main`, no series, nothing else
    /// moved, under the subject reserved to the bot.
    #[test]
    fn a_group_only_cut_opens_no_series() {
        let groups = workspace_groups();
        let bedrock = decide(
            "bedrock",
            "0.3.0",
            &["bedrock-v0.3.0"],
            &[library_commit("bedrock", "fix(bedrock): a bug")],
            None,
        );
        let plan = Plan::library(bedrock);
        assert!(plan.is_group_only());
        assert_eq!(plan.landing(&groups).unwrap(), None);
        assert_eq!(plan.group_tags, "bedrock-v0.3.1");
        let main = plan.main_landing(&groups).unwrap().unwrap();
        assert_eq!(main.title, "chore(release): bedrock 0.3.1");
        assert_eq!(
            main.paths,
            "crates/bedrock/Cargo.toml,crates/bedrock-core/Cargo.toml,crates/bedrock-derive/Cargo.toml,Cargo.lock"
        );
        assert_eq!(
            plan.summary_headers(&groups).unwrap(),
            ["chore(bedrock): 0.3.1"]
        );
    }

    /// Scenario _A floor raises a group's bump_, and never lowers it.
    #[test]
    fn a_floor_raises_a_group_s_bump_and_never_lowers_it() {
        let fixes = [library_commit("outpost", "fix(outpost): a bug")];
        let raised = decide("outpost", "1.4.2", &[], &fixes, Some(Floor::Major));
        assert_eq!(raised.next.as_deref(), Some("2.0.0"));

        let breaking = [library_commit("outpost", "feat(outpost)!: a break")];
        let same = decide("outpost", "1.4.2", &[], &breaking, Some(Floor::Major));
        assert_eq!(same.next.as_deref(), Some("2.0.0"));

        let feature = [library_commit("outpost", "feat(outpost): a feature")];
        let kept = decide("outpost", "1.4.2", &[], &feature, Some(Floor::Patch));
        assert_eq!(kept.next.as_deref(), Some("1.5.0"));
    }

    /// Scenario _Nothing to cut_: a group with no counted commit mints
    /// nothing, floor or not, and a cut scoped to it is refused by name.
    #[test]
    fn nothing_to_cut_mints_nothing() {
        let domain = decide(
            "domain",
            "0.1.1",
            &["domain-v0.1.1"],
            &[],
            Some(Floor::Major),
        );
        assert!(!domain.mints());
        assert_eq!(domain.counted, 0);
        assert!(
            domain.describe().contains("unchanged"),
            "{}",
            domain.describe()
        );
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
            let landing = plan.landing(&groups).unwrap().unwrap();
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
        let series = cut.landing(&groups).unwrap().unwrap();
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
