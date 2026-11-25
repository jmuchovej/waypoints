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
    constants::{
        APP_MANIFEST, GROUP_FILE, LOCKFILE, MAIN, RELEASE_PLAN_HEAD_PREFIX, REPOSITORY,
        ROOT_MANIFEST,
    },
    planners::test,
    release_groups::{self, GroupFile, pins_in},
    releases::{
        history,
        landing::Landing,
        version::{Transition, Version},
    },
    semver_check::{self, Required},
    templates,
    tools::{git, github},
    workspace,
};
use anyhow::{Context as _, Result, bail};
use cargo_metadata::{
    Metadata,
    semver::{Comparator, Op, Prerelease},
};
use git_conventional::Commit as Conventional;
use indoc::formatdoc;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};
use tera::{Context, Value as TeraValue, context};
use toml_edit::{DocumentMut, Item, RawString, Value};

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
    /// The version the group file declares: what the group is now.
    pub current: String,
    /// What it becomes, or none when no counted commit landed since its tag.
    pub next: Option<String>,
    /// The tag a cut mints for it, when `next` is some.
    pub tag: Option<String>,
    /// The tag it was last minted at, or none before its first cut.
    pub last_tag: Option<String>,
    /// The commits that counted for it since that tag.
    pub counted: usize,
    /// What the API check found against `last_tag`: a break its commits did
    /// not declare raises the bump to breaking, a surface that grew to a
    /// feature (design D5). None where nothing was measured — no tag to
    /// measure against, or nothing counted.
    pub api: Option<Required>,
    /// Whether a frozen fixture of the group changed since the last stable
    /// release, which is breaking for the group whatever its commits say.
    pub fixture_break: bool,
}

/// The headers the floors speak in: a break and a feature, as the bumper
/// reads them.
const DETECTED_BREAK: &str = "feat!: a break the evidence shows";
const DETECTED_GROWTH: &str = "feat: growth the evidence shows";

/// What raises a group's bump beyond what its commits declare; none of it
/// ever lowers the bump.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Raise {
    /// The least a group-scoped cut bumps.
    pub floor: Option<Floor>,
    /// What the API check found against the group's last tag: a break
    /// raises the bump to breaking, a surface that grew to a feature (design
    /// D5). None where nothing was measured — no tag to measure against, or
    /// nothing counted.
    pub detected: Option<Required>,
    /// Whether a frozen fixture of the group was modified, deleted, or
    /// renamed since the last stable release: breaking, since it changes
    /// what old data means (design D5).
    pub fixtures: bool,
}

impl GroupPlan {
    /// Decides `group` from its tag `template`, its `current` version, the
    /// remote's `tags`, and the `commits` since its last tag, raised by
    /// `raise` — the floor a group-scoped cut names, and the evidence of the
    /// API check and the fixtures. A group with no counted commit stays where
    /// it is; before its first tag, the file's declaration is its version and
    /// the whole history its cycle, as the app's first cut is measured, and
    /// there is no baseline to measure against.
    pub fn decide(
        group: &str,
        template: &str,
        current: &Version,
        tags: &[String],
        commits: &[history::Commit],
        groups: &GroupFile,
        raise: Raise,
    ) -> Result<Self> {
        let prefix = tag_prefix(template)?;
        let last = latest_group_tag(prefix, tags);
        let headers: Vec<String> = commits
            .iter()
            .filter_map(|commit| commit.header_for(group, groups))
            .collect();
        let counted = headers.len();
        let next = (counted > 0).then(|| {
            let mut bumped = current.next_library(&headers);
            let evidence = match raise.detected {
                Some(Required::Major) => Some(DETECTED_BREAK),
                Some(Required::Minor) => Some(DETECTED_GROWTH),
                _ => None,
            };
            if let Some(header) = evidence {
                bumped = bumped.max(current.next_library(&[header.to_owned()]));
            }
            if raise.fixtures {
                bumped = bumped.max(current.next_library(&[DETECTED_BREAK.to_owned()]));
            }
            match raise.floor {
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
            api: raise.detected,
            fixture_break: raise.fixtures,
        })
    }

    /// Whether a cut mints a tag for this group.
    pub fn mints(&self) -> bool {
        self.next.is_some()
    }

    /// One line a reader audits before a cut.
    pub fn describe(&self) -> String {
        let api = match (&self.last_tag, self.api) {
            (None, _) => "; no tag to measure an API break against until this cut mints one",
            (Some(_), Some(Required::Major)) => "; the API check found a break",
            (Some(_), Some(Required::Minor)) => "; the API check found the surface grew",
            (Some(_), Some(Required::Patch)) => "; the API check found no break",
            (Some(_), None) => "",
        };
        let fixtures = if self.fixture_break {
            "; a frozen fixture changed since the last stable release"
        } else {
            ""
        };
        let suffix = if self.mints() { "minted" } else { "unchanged" };
        let template = &format!("dry-run.group-{suffix}");
        let since = match &self.last_tag {
            Some(tag) => format!("since `{tag}`"),
            None => "over the whole history, with no tag yet".to_owned(),
        };
        templates::must(
            template,
            context! {
              group => &self.group,
              current => &self.current,
              next => self.next.as_deref().unwrap_or_default(),
              tag => self.tag.as_deref().unwrap_or_default(),
              counted => &self.counted,
              since,
              evidence => &format!("{api}{fixtures}"),
            },
        )
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

/// One pinned library group's part of a patch: the tag the series pins, the
/// newer tag on the pin's line it moves to, or why it stays (design D1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinMove {
    pub group: String,
    /// The tag the series pins now.
    pub from: String,
    /// The newest tag on the pin's line, where one is newer than the pin.
    pub to: Option<String>,
    /// The commits on `main` that broke the group's line since the newest
    /// tag on it — `<sha> <subject>` — which no tag on that line can carry.
    pub breaking: Vec<String>,
}

impl PinMove {
    /// Decides `group`, pinned at `from`, moving to `to` where the remote
    /// holds a newer tag on the pin's line, from the `commits` on `main`
    /// since that tag — or since the pin, where there is none: a break among
    /// them is named, and with nothing to move to it is the refusal, since a
    /// fix landed after a break reaches a series only by a new cut.
    pub fn decide(
        group: &str,
        from: &str,
        to: Option<String>,
        commits: &[history::Commit],
        groups: &GroupFile,
    ) -> Self {
        let breaking = commits
            .iter()
            .filter(|commit| {
                commit.header_for(group, groups).is_some_and(|header| {
                    Conventional::parse(&header).is_ok_and(|header| header.breaking())
                })
            })
            .map(|commit| {
                format!(
                    "{} {}",
                    &commit.sha[..commit.sha.len().min(9)],
                    commit.subject
                )
            })
            .collect();
        Self {
            group: group.to_owned(),
            from: from.to_owned(),
            to,
            breaking,
        }
    }

    /// Whether the patch moves this pin.
    pub fn moves(&self) -> bool {
        self.to.is_some()
    }

    /// The tag the series builds the group from after the patch.
    pub fn target(&self) -> &str {
        self.to.as_deref().unwrap_or(&self.from)
    }

    /// One line a reader audits before a patch.
    pub fn describe(&self) -> String {
        let template = match (&self.to, self.breaking.is_empty()) {
            (Some(_), _) => "dry-run.pin-moved",
            (None, true) => "dry-run.pin-held",
            (None, false) => "dry-run.pin-refused",
        };
        templates::must(
            template,
            context! {
                group => &self.group,
                from => &self.from,
                to => self.to.as_deref().unwrap_or_default(),
                breaking => &self.breaking,
            },
        )
    }
}

/// The newest of a group's `tags` on `pinned`'s line, where one is newer
/// than the pin. The line is what cargo reads a caret requirement as — the
/// same major, or before `1.0` the same minor, and before `0.1` nothing but
/// itself — so it is `semver`'s own `^pinned`.
fn newer_on_line(prefix: &str, pinned: &Version, tags: &[String]) -> Option<Version> {
    let line = Comparator {
        op: Op::Caret,
        major: pinned.major,
        minor: Some(pinned.minor),
        patch: Some(pinned.patch),
        pre: Prerelease::EMPTY,
    };
    tags.iter()
        .filter_map(|tag| tag.strip_prefix(prefix))
        .filter_map(|rest| Version::parse(rest).ok())
        .filter(|candidate| candidate > pinned && line.matches(candidate))
        .max()
}

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
    /// Every pinned library group's part of a patch, moving or not. Not an
    /// output: the planner writes the moves itself.
    #[serde(skip)]
    pub moves: Vec<PinMove>,
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
            moves: Vec::new(),
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
            moves: Vec::new(),
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

    /// The same plan, carrying every pinned group's part of the patch.
    pub fn with_moves(mut self, moves: Vec<PinMove>) -> Self {
        self.moves = moves;
        self
    }

    /// Whether this patch moves a pin.
    pub fn moves_a_pin(&self) -> bool {
        self.moves.iter().any(PinMove::moves)
    }

    /// Whether this cut mints a library alone, with no app version behind it.
    pub fn is_group_only(&self) -> bool {
        self.tag.is_empty()
    }

    /// Whether this plan opens a series: a cut with an app version, which
    /// also moves `main`.
    pub fn opens_series(&self) -> bool {
        !self.next_main_version.is_empty()
    }

    /// The groups this plan mints.
    pub fn minted(&self) -> impl Iterator<Item = &GroupPlan> {
        self.groups.iter().filter(|group| group.mints())
    }

    /// What this plan's series write pins: on a cut, each library group at
    /// the tag the cut mints, or at its last one where nothing counted — a
    /// group with no tag at all stays in-tree; on a patch, each pin it
    /// moves, at the tag it moves to.
    pub fn pins(&self) -> BTreeMap<String, String> {
        self.groups
            .iter()
            .filter_map(|group| {
                let tag = group.tag.clone().or_else(|| group.last_tag.clone())?;
                Some((group.group.clone(), tag))
            })
            .chain(
                self.moves
                    .iter()
                    .filter_map(|pin| Some((pin.group.clone(), pin.to.clone()?))),
            )
            .collect()
    }

    /// The message for the version commit this plan's tag points at.
    pub fn commit_message(&self) -> String {
        templates::must(
            "commit.series",
            context! {
                branch => &self.branch,
                channel => &self.channel,
                version => &self.version,
            },
        )
    }

    /// The pull request that lands this plan's version write on its series —
    /// and, where the write also pins (the series a cut opens) or moves a
    /// pin (a patch), the root manifest. None on a group-scoped cut, which
    /// opens no series.
    pub fn landing(&self, groups: &GroupFile) -> Result<Option<Landing>> {
        if self.is_group_only() {
            return Ok(None);
        }
        Ok(Some(Landing::new(
            format!("{RELEASE_PLAN_HEAD_PREFIX}{}", self.branch),
            self.commit_message(),
            &written_paths(groups, self.opens_series() || self.moves_a_pin())?,
        )))
    }

    /// The pull request that lands the `main`-side writes: the app's bump,
    /// each minted group's version, or both — in the group file and the
    /// manifests projecting it. None when nothing lands there.
    pub fn main_landing(&self, groups: &GroupFile) -> Result<Option<Landing>> {
        let minted: Vec<&GroupPlan> = self.minted().collect();
        if self.next_main_version.is_empty() && minted.is_empty() {
            return Ok(None);
        }

        let mut paths = vec![GROUP_FILE.to_owned()];
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
            format!("{RELEASE_PLAN_HEAD_PREFIX}{MAIN}"),
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
            let ctx = context! {
                group => &only.group,
                version => only.next.as_deref().unwrap_or_default(),
            };
            return Ok(templates::must("commit.library", ctx));
        }
        let ctx = context! {
          version => &self.version,
        };
        Ok(templates::must("commit.summary", ctx))
    }

    /// The headers a summary's body carries, one per group it writes, scoped
    /// by the group so the pull-request check reads each.
    pub fn summary_headers(&self, groups: &GroupFile) -> Result<Vec<String>> {
        let mut headers: Vec<String> = self
            .minted()
            .map(|group| {
                let ctx = context! {
                    group => &group.group,
                    version => group.next.as_deref().unwrap_or_default(),
                };
                templates::must("header.group", ctx)
            })
            .collect();
        if !self.next_main_version.is_empty() {
            let ctx = context! {
                app => app_group(groups)?,
                next => &self.next_main_version,
            };
            headers.push(templates::must("header.main", ctx));
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

/// The crates an app version write sets together: the app's group, which the
/// pull-request check holds to one version.
fn app_members(groups: &GroupFile) -> Result<&[String]> {
    members_of(groups, groups.app_group()?)
}

/// What a version write changes: the group file that declares the version,
/// every member's manifest that projects it, the root manifest where the
/// write also `pins` the series' libraries, and the lockfile entries that
/// repeat their versions.
fn written_paths(groups: &GroupFile, pins: bool) -> Result<Vec<String>> {
    let mut paths = vec![GROUP_FILE.to_owned()];
    paths.extend(
        app_members(groups)?
            .iter()
            .map(|member| format!("{member}/Cargo.toml")),
    );
    if pins {
        paths.push(ROOT_MANIFEST.to_owned());
    }
    paths.push(LOCKFILE.to_owned());
    Ok(paths)
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
    let ctx = context! {
      next => &next.to_string(),
    };
    templates::must("commit.main", ctx)
}

/// The refusal for a cut whose cycle holds no commit counting for the app:
/// what the cut is measured from, and which libraries moved, which a
/// group-scoped cut mints alone.
fn app_unmoved(app: &str, last: Option<&Version>, planned: &[GroupPlan]) -> String {
    let since = match last {
        Some(last) => format!("since `{}`", last.tag()),
        None => "in its history".to_owned(),
    };
    let moved: Vec<String> = planned
        .iter()
        .filter(|group| group.mints())
        .map(|group| format!("`{}`", group.group))
        .collect();
    templates::must(
        "refusal.app-unmoved",
        context! {
            app,
            since,
            moved
        },
    )
}

/// Each library group the series `branch` pins, carried forward along its
/// line: the pins read from the series' root manifest as the remote has it,
/// the line's newer tags from the remote, and the break check from `main`'s
/// history since the newest tag on the line.
fn pin_moves(branch: &str, groups: &GroupFile, root: &Path) -> Result<Vec<PinMove>> {
    let manifest = git::show(&remote_or_local(branch), ROOT_MANIFEST)?;
    let mut pinned: BTreeMap<String, String> = BTreeMap::new();
    for pin in pins_in(ROOT_MANIFEST, &manifest, groups)? {
        pinned.entry(pin.group).or_insert(pin.tag);
    }
    let ignored = history::ignored_revisions(root, groups)?;
    let main = remote_or_local(MAIN);
    let mut moves = Vec::new();
    for (group, from) in pinned {
        let template = &groups
            .group
            .get(&group)
            .with_context(|| format!("no group named `{group}`"))?
            .tag;
        let prefix = tag_prefix(template)?;
        let version = from
            .strip_prefix(prefix)
            .map(Version::parse)
            .transpose()?
            .with_context(|| format!("`{from}` is not a `{template}` tag"))?;
        let tags = git::remote_tags_like(&format!("{prefix}*"))?;
        let to = newer_on_line(prefix, &version, &tags).map(|to| format!("{prefix}{to}"));
        let since = to.as_deref().unwrap_or(&from);
        let commits = history::walk(&format!("{since}..{main}"), groups, &ignored)?;
        moves.push(PinMove::decide(&group, &from, to, &commits, groups));
    }
    Ok(moves)
}

/// Plans `transition` from the checkout: the app's current version, or for
/// a cut the stable tags and the commits since the last one — and for a
/// patch, each library pin's move along its line. Refuses a tree the group
/// file does not describe.
pub fn plan(transition: Transition, options: CutOptions) -> Result<Plan> {
    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let root = metadata.workspace_root.as_std_path();

    if transition != Transition::Cut {
        let version = groups.app_version()?.transition(transition, &[])?;
        let plan = Plan::from_version(transition, version)?;
        // A patch carries each pin forward along its line; advance and
        // promote move the app alone.
        if transition != Transition::Patch {
            return Ok(plan);
        }
        let moves = pin_moves(&plan.branch, &groups, root)?;
        return Ok(plan.with_moves(moves));
    }

    let ignored = history::ignored_revisions(root, &groups)?;
    let app = app_group(&groups)?;

    // The fixture floor: a frozen fixture modified, deleted, or renamed since
    // the last stable release breaks its group and the app. Before the first
    // stable release there is nothing to diff from.
    let last = last_stable(&stable_tags()?);
    let fixtures = match &last {
        Some(last) => fixture_breaks(&crate::version_integrity::changes(&last.tag())?, &groups),
        None => BTreeMap::new(),
    };

    // One library alone: its tag on `main`, nothing else moved, and a refusal
    // where nothing counted since its last tag.
    if let Some(name) = &options.group {
        if name == app {
            bail!("`{name}` is the app; a cut moves it with its series, never alone");
        }
        if !groups.group.contains_key(name) {
            bail!("no group named `{name}` in {GROUP_FILE}");
        }
        let only = group_plan(
            name,
            &groups,
            &metadata,
            &ignored,
            options.floor,
            fixtures.contains_key(name),
        )?;
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
        .map(|name| {
            group_plan(
                name,
                &groups,
                &metadata,
                &ignored,
                None,
                fixtures.contains_key(name),
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let commits = commits_since(last.as_ref(), &groups, root)?;
    // Nothing counted for the app since the last stable release: there is no
    // app to cut, whatever the libraries did, and a target computed from no
    // commit would sit below that release. Before `--major` is read, since a
    // major is a floor on counted work, not a substitute for it.
    if !commits.iter().any(|commit| commit.counts_for(app)) {
        bail!("{}", app_unmoved(app, last.as_ref(), &planned));
    }
    // Nothing released yet: the first cut is measured from 0.0.0, and
    // `log_range` reads the whole history rather than a missing tag.
    let last = last.unwrap_or_else(|| Version::new(0, 0, 0));
    let changed: Vec<String> = fixtures.values().flatten().cloned().collect();
    let version = cut_target(&last, &commits, &groups, options.major, &changed)?;
    Ok(Plan::from_version(Transition::Cut, version)?.with_groups(planned))
}

/// The frozen fixtures among `changes` that were modified or deleted — a
/// rename is both — by the group owning them: breaking for that group, and
/// for the app, which ships them all. A group's own `README.md` is not a
/// fixture, and an added fixture breaks nothing.
fn fixture_breaks(
    changes: &[crate::version_integrity::Change],
    groups: &GroupFile,
) -> BTreeMap<String, Vec<String>> {
    use crate::version_integrity::{Status, is_fixture};
    let mut broken: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for change in changes {
        if change.status == Status::Added || !is_fixture(&change.path) {
            continue;
        }
        if let Some(owner) = groups.owner_of(&change.path) {
            broken
                .entry(owner.to_owned())
                .or_default()
                .push(change.path.clone());
        }
    }
    broken
}

/// One library group's part of a cut, from the remote's tags and the commits
/// since its last one, raised by the API check against that tag and by a
/// frozen `fixture` of its own changed since the last stable release.
fn group_plan(
    name: &str,
    groups: &GroupFile,
    metadata: &Metadata,
    ignored: &BTreeSet<String>,
    floor: Option<Floor>,
    fixtures: bool,
) -> Result<GroupPlan> {
    let group = &groups.group[name];
    let prefix = tag_prefix(&group.tag)?;
    let current = groups.version_of(name)?;
    let tags = git::remote_tags_like(&format!("{prefix}*"))?;
    let last = latest_group_tag(prefix, &tags);
    let commits = history::walk(&group_range(prefix, last.as_ref()), groups, ignored)?;
    // The API floor: measured against the last tag where there is one and
    // something counted since it; a group with no tag has no baseline until
    // this cut mints one, and says so in its line.
    let detected = match &last {
        Some(last) if commits.iter().any(|commit| commit.counts_for(name)) => {
            let tag = format!("{prefix}{last}");
            let measured = semver_check::measure_group(metadata, groups, name, &tag)?;
            // The group requires what its most demanding member does.
            let required = measured.into_iter().map(|(_, required)| required).max();
            Some(required.unwrap_or(Required::Patch))
        }
        _ => None,
    };
    GroupPlan::decide(
        name,
        &group.tag,
        &current,
        &tags,
        &commits,
        groups,
        Raise {
            floor,
            detected,
            fixtures,
        },
    )
}

/// The app's target for a cut from `last` over `commits`, with the frozen
/// `fixtures` changed since `last`.
///
/// A major is cut only when asked for: it is a product decision no commit
/// can prove. A `Covenant: break` since the last stable is the floor, not the
/// trigger — and so is a changed fixture, which changes what old data means
/// — without `--major` the cut refuses and names them, since a silent minor
/// and an unreviewed major are both wrong. The app's headers are read as
/// [`history::Commit::header_for`] reads them, so a `!` never reaches the
/// bumper and the trailer does.
pub fn cut_target(
    last: &Version,
    commits: &[history::Commit],
    groups: &GroupFile,
    major: bool,
    fixtures: &[String],
) -> Result<Version> {
    let app = app_group(groups)?;
    if major {
        return last.cut_major();
    }

    let mut breaking: Vec<String> = commits
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
    breaking.extend(
        fixtures
            .iter()
            .map(|path| format!("`{path}`, a frozen fixture, changed")),
    );
    if !breaking.is_empty() {
        bail!(formatdoc! { "
            a covenant break landed since {last}:
              {breaking}
            a major is a product decision: pass `--major` to cut {next}, or hold the cut",
            breaking = breaking.join("\n  "),
            next = last.cut_major()?,
        });
    }

    let headers: Vec<String> = commits
        .iter()
        .filter_map(|commit| commit.header_for(app, groups))
        .collect();
    last.cut(&headers)
}

/// The app's group — the one owning the remainder — whose version a cut
/// moves with the series.
fn app_group(groups: &GroupFile) -> Result<&str> {
    groups.app_group()
}

/// Leaves `plan`'s writes in the working tree for the pull-request step to
/// commit, each a projection of the group file onto its members: on a cut,
/// the `main`-side versions — each minted group's and the app's bump — on
/// `main`; otherwise the app's version on its series, and on a patch the pins
/// it moves, each moved group's line in the file beside the root manifest's
/// pin. A cut's series is opened later, from the merge commit those writes
/// land as ([`open_series`]).
pub fn execute(transition: Transition, plan: &Plan) -> Result<()> {
    if transition == Transition::Cut {
        git::check_out_latest(MAIN)?;
        let metadata = workspace::load()?;
        let groups = release_groups::load(&metadata)?;
        let mut writes = Vec::new();
        for group in plan.minted() {
            let next = Version::parse(group.next.as_deref().unwrap_or_default())?;
            writes.push((group.group.clone(), next));
        }
        if !plan.next_main_version.is_empty() {
            let next = Version::parse(&plan.next_main_version)?;
            writes.push((app_group(&groups)?.to_owned(), next));
        }
        project(&metadata, &groups, &writes)?;
        return Ok(());
    }

    git::check_out_latest(&plan.branch)?;
    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let root = metadata.workspace_root.as_std_path();
    let mut writes = vec![(
        app_group(&groups)?.to_owned(),
        Version::parse(&plan.version)?,
    )];
    if plan.moves_a_pin() {
        let manifest = root.join(ROOT_MANIFEST);
        let pins = plan.pins();
        write(
            &manifest,
            &pin_workspace(&read(&manifest)?, &pins, &groups)?,
        )?;
        for (group, tag) in &pins {
            writes.push((group.clone(), tag_version(&groups, group, tag)?));
        }
    }
    project(&metadata, &groups, &writes)?;
    if plan.moves_a_pin() {
        update_lockfile(root)?;
    }
    Ok(())
}

/// Opens a cut's series at `commit` — the merge commit of its main pull
/// request, which its first-parent history contains — and leaves its first
/// write in the working tree for the pull-request step, which reads the
/// returned landing: every library group pinned to its newest tag on the
/// remote, which the cut's tagging step moved for each group it minted
/// (design D7), and the group file saying what the series ships — the app's
/// `version`, projected into its manifests, and each pinned group's held to
/// its pin.
pub fn open_series(version: &Version, commit: &str) -> Result<Landing> {
    let plan = Plan::from_version(Transition::Cut, version.clone())?;
    github::create_branch(&plan.branch, commit)?;
    git::check_out_latest(&plan.branch)?;
    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let root = metadata.workspace_root.as_std_path();

    let manifest = root.join(ROOT_MANIFEST);
    let pins = remote_pins(&groups)?;
    write(
        &manifest,
        &pin_workspace(&read(&manifest)?, &pins, &groups)?,
    )?;
    let mut writes = vec![(app_group(&groups)?.to_owned(), version.clone())];
    for (group, tag) in &pins {
        writes.push((group.clone(), tag_version(&groups, group, tag)?));
    }
    project(&metadata, &groups, &writes)?;
    update_lockfile(root)?;
    plan.landing(&groups)?.context("a cut opens a series")
}

/// Each library group at its newest tag on the remote. A group with no tag
/// is absent: it stays in-tree, as design D8 reads it.
fn remote_pins(groups: &GroupFile) -> Result<BTreeMap<String, String>> {
    let app = app_group(groups)?;
    let mut pins = BTreeMap::new();
    for (name, group) in groups.group.iter().filter(|(name, _)| *name != app) {
        let prefix = tag_prefix(&group.tag)?;
        let tags = git::remote_tags_like(&format!("{prefix}*"))?;
        if let Some(latest) = latest_group_tag(prefix, &tags) {
            pins.insert(name.clone(), format!("{prefix}{latest}"));
        }
    }
    Ok(pins)
}

/// The root `manifest` as a series carries it: every workspace dependency on
/// a crate of a pinned group declared as `git` + `tag` in place of `path`,
/// every other key kept, and those crates out of `[workspace].members` under
/// a comment saying why — they stay on disk, and build nothing. A dependency
/// already pinned to the group moves to the group's tag, which is how a patch
/// carries a pin forward. `pins` maps a group to its tag; a group absent from
/// it stays as it is.
///
/// Edited as a document, not reformatted: a key keeps its indent and the
/// alignment tombi gave it, and a pinned entry stays on its line, since the
/// root manifest is TOML 1.0 for nix's sake and an inline table there cannot
/// break across lines.
fn pin_workspace(
    manifest: &str,
    pins: &BTreeMap<String, String>,
    groups: &GroupFile,
) -> Result<String> {
    let mut document: DocumentMut = manifest
        .parse()
        .with_context(|| format!("{ROOT_MANIFEST} does not parse"))?;
    let workspace = document
        .get_mut("workspace")
        .and_then(Item::as_table_mut)
        .with_context(|| format!("{ROOT_MANIFEST} has no [workspace]"))?;

    // Each pinned group's crates by directory, and its tags by prefix.
    let mut crates: BTreeMap<&str, &str> = BTreeMap::new();
    let mut prefixes: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, tag) in pins {
        let group = groups
            .group
            .get(name)
            .with_context(|| format!("no group named `{name}`"))?;
        for directory in group.members.iter().chain(&group.internals) {
            crates.insert(directory, tag);
        }
        prefixes.insert(tag_prefix(&group.tag)?, tag);
    }

    let dependencies = workspace
        .get_mut("dependencies")
        .and_then(Item::as_table_like_mut)
        .with_context(|| format!("{ROOT_MANIFEST} has no [workspace.dependencies]"))?;
    for (_, item) in dependencies.iter_mut() {
        let Some(spec) = item.as_inline_table_mut() else {
            continue;
        };
        if let Some(tag) = spec
            .get("path")
            .and_then(Value::as_str)
            .and_then(|path| crates.get(path))
        {
            let kept: Vec<(String, Value)> = spec
                .iter()
                .filter(|(key, _)| *key != "path")
                .map(|(key, value)| (key.to_owned(), value.clone()))
                .collect();
            spec.clear();
            spec.insert("git", REPOSITORY.into());
            spec.insert("tag", (*tag).into());
            for (key, value) in kept {
                spec.insert(key, value);
            }
            spec.fmt();
            continue;
        }
        let moved = spec
            .get("tag")
            .and_then(Value::as_str)
            .and_then(|current| {
                prefixes
                    .iter()
                    .find(|(prefix, _)| current.starts_with(*prefix))
            })
            .map(|(_, tag)| *tag);
        if let Some(tag) = moved {
            spec.insert("tag", tag.into());
        }
    }

    let members = workspace
        .get_mut("members")
        .and_then(Item::as_array_mut)
        .with_context(|| format!("{ROOT_MANIFEST} has no [workspace].members"))?;
    let before = members.len();
    members.retain(|member| {
        member
            .as_str()
            .is_none_or(|directory| !crates.contains_key(directory))
    });
    if members.len() != before
        && let Some(mut key) = workspace.key_mut("members")
    {
        // The comment goes above the key, under whatever already sits there,
        // at the key's own indent.
        let decor = key.leaf_decor_mut();
        let prefix = decor
            .prefix()
            .and_then(RawString::as_str)
            .unwrap_or_default()
            .to_owned();
        let (above, indent) = match prefix.rsplit_once('\n') {
            Some((above, indent)) => (format!("{above}\n"), indent.to_owned()),
            None => (String::new(), prefix),
        };
        let comment: String = templates::must("manifest.pinned", Context::new())
            .lines()
            .map(|line| format!("{indent}{line}\n"))
            .collect();
        decor.set_prefix(format!("{above}{comment}{indent}"));
    }

    Ok(document.to_string())
}

/// Re-resolves the lockfile for the manifests as they are now. `--workspace`
/// touches only what the workspace's own manifests changed: the pinned
/// crates enter from their tags and leave as members, and nothing else
/// moves. Fetches the tags, so it runs where the remote is reachable.
fn update_lockfile(root: &Path) -> Result<()> {
    let status = Command::new("cargo")
        .args(["update", "--workspace"])
        .current_dir(root)
        .status()
        .context("running `cargo update --workspace`")?;
    if !status.success() {
        bail!("`cargo update --workspace` failed with {status}");
    }
    Ok(())
}

/// The body of every pull request the planner opens. It becomes the merge
/// commit's body, so it says what the merge commit is; GitHub wraps it at 72
/// columns, so it stays under them.
pub fn pull_request_body() -> String {
    templates::must("pull-request.plan", Context::new())
}

/// What a dry run renders instead of writing, each file by name, for the task
/// to put under one directory: `series.patch` and, for a cut, `main.patch` are
/// the version writes as unified diffs against the trees they would land on —
/// each the group file's lines beside the manifests they project into; a
/// cut's `series.patch` carries the root manifest's pins too, and each pinned
/// group's line in the file, though not the lockfile entries cargo resolves
/// from the pins once the tags exist; `refs.md` names the branch, the tag,
/// and the pull requests; `notes.md` is git-cliff's rendering for the
/// would-be tag. Everything is read with `git show`, so the checkout is left
/// as it was.
pub fn rehearse(transition: Transition, plan: &Plan) -> Result<Vec<(String, String)>> {
    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let packages = test::package_directories(&metadata);
    let app = app_group(&groups)?;
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
        let mut writes = vec![(app.to_owned(), Version::parse(&plan.version)?)];
        let mut changes = Vec::new();
        // The pins a cut starts its series on, or the ones a patch moves: the
        // root manifest as the write leaves it, which the projection reads to
        // tell a pinned group — its line in the file alone — from one in tree.
        let mut root_manifest = git::show(&series_tree, ROOT_MANIFEST)?;
        if plan.opens_series() || plan.moves_a_pin() {
            let pins = plan.pins();
            let after = pin_workspace(&root_manifest, &pins, &groups)?;
            for (group, tag) in &pins {
                writes.push((group.clone(), tag_version(&groups, group, tag)?));
            }
            changes.push((ROOT_MANIFEST.to_owned(), root_manifest, after.clone()));
            root_manifest = after;
        }
        changes.extend(projection(&groups, &packages, &writes, |path| {
            if path == ROOT_MANIFEST {
                Ok(root_manifest.clone())
            } else {
                git::show(&series_tree, path)
            }
        })?);
        files.push(("series.patch".to_owned(), unified_diff(&changes)?));
    }

    // The main-side commit: the app's bump, each minted group's version, or
    // both.
    let main = plan.main_landing(&groups)?;
    if main.is_some() {
        let mut writes = Vec::new();
        if !plan.next_main_version.is_empty() {
            writes.push((app.to_owned(), Version::parse(&plan.next_main_version)?));
        }
        for group in plan.minted() {
            let next = Version::parse(group.next.as_deref().unwrap_or_default())?;
            writes.push((group.group.clone(), next));
        }
        let changes = projection(&groups, &packages, &writes, |path| git::show("HEAD", path))?;
        files.push(("main.patch".to_owned(), unified_diff(&changes)?));
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

/// The remote's view of `branch` where the checkout has fetched it, else the
/// local branch — and a name `git show` fails on, by name, when neither exists.
fn remote_or_local(branch: &str) -> String {
    let remote = format!("origin/{branch}");
    match git::run(&["rev-parse", "--verify", "--quiet", &remote]) {
        Ok(_) => remote,
        Err(_) => branch.to_owned(),
    }
}

/// `changes` — `(path, before, after)` — as `git diff` prints them between two
/// trees, from a scratch directory holding both sides under `a/` and `b/`.
fn unified_diff(changes: &[(String, String, String)]) -> Result<String> {
    // One directory per call, not per process: two rehearsals in one process
    // — the test runner's threads — would otherwise write over each other.
    static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let scratch =
        std::env::temp_dir().join(format!("xtask-release-plan-{}-{call}", std::process::id()));
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
    // The nested tables are `Value` maps: the macro inserts whatever
    // serializes, and a map of values is what `main.next` reads from.
    let main = match main {
        Some(landing) => {
            let next = (!plan.next_main_version.is_empty()).then(|| plan.next_main_version.clone());
            let headers = if landing.title.starts_with("summary:") {
                plan.summary_headers(groups)?
            } else {
                Vec::new()
            };
            Some(TeraValue::from(BTreeMap::from([
                ("next", TeraValue::from(next)),
                ("landing", TeraValue::from(landing.describe(MAIN))),
                ("headers", TeraValue::from(headers)),
            ])))
        }
        None => None,
    };
    let series = series.map(|landing| {
        TeraValue::from(BTreeMap::from([
            ("branch", TeraValue::from(plan.branch.clone())),
            ("created", TeraValue::from(plan.opens_series())),
            ("tag", TeraValue::from(plan.tag.clone())),
            ("landing", TeraValue::from(landing.describe(&plan.branch))),
        ]))
    });
    let groups: Vec<String> = plan
        .groups
        .iter()
        .map(GroupPlan::describe)
        .chain(plan.moves.iter().map(PinMove::describe))
        .collect();
    templates::render(
        "dry-run.refs",
        context! {
            transition => &format!("{transition:?}").to_lowercase(),
            body => &pull_request_body(),
            series,
            main,
            groups,
        },
    )
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
            api: None,
            fixture_break: false,
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
        assert_eq!(refs.matches(&pull_request_body()).count(), 2, "{refs}");
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

/// The texts a version write rewrites, as `(path, before, after)`, read
/// through `read` — the checkout, or a tree `git show` reads — and left for
/// the caller to write or to diff: for each `(group, version)` of `writes`,
/// the group's `version` line in the group file and, unless the root manifest
/// pins the group — a series' library, on disk and out of the workspace,
/// whose line is held to its pin — every member's manifest and lockfile
/// entry. The file comes first and the lockfile last, each carrying every
/// write.
fn projection(
    groups: &GroupFile,
    packages: &[(String, String)],
    writes: &[(String, Version)],
    read: impl Fn(&str) -> Result<String>,
) -> Result<Vec<(String, String, String)>> {
    let pinned: BTreeSet<String> = pins_in(ROOT_MANIFEST, &read(ROOT_MANIFEST)?, groups)?
        .into_iter()
        .map(|pin| pin.group)
        .collect();
    let file = read(GROUP_FILE)?;
    let mut file_after = file.clone();
    let lock = read(LOCKFILE)?;
    let mut lock_after = lock.clone();
    let mut manifests = Vec::new();
    for (group, version) in writes {
        file_after = set_group_version(&file_after, group, version)?;
        if pinned.contains(group) {
            continue;
        }
        for member in members_of(groups, group)? {
            let path = format!("{member}/Cargo.toml");
            let manifest = read(&path)?;
            let rewritten = set_package_version(&manifest, version)?;
            manifests.push((path, manifest, rewritten));

            let (package, _) = packages
                .iter()
                .find(|(_, directory)| directory == member)
                .with_context(|| format!("`{member}` is not a workspace member"))?;
            lock_after = set_locked_version(&lock_after, package, version)?;
        }
    }
    let mut changes = vec![(GROUP_FILE.to_owned(), file, file_after)];
    changes.extend(manifests);
    changes.push((LOCKFILE.to_owned(), lock, lock_after));
    Ok(changes)
}

/// Writes each `(group, version)` of `writes` in the tree checked out now —
/// the group file's line, the members' manifests, their lockfile entries, as
/// [`projection`] reads them — and returns the paths it changed. A file the
/// write leaves as it is stays untouched.
fn project(
    metadata: &Metadata,
    groups: &GroupFile,
    writes: &[(String, Version)],
) -> Result<Vec<String>> {
    let root = metadata.workspace_root.as_std_path();
    let packages = test::package_directories(metadata);
    let changes = projection(groups, &packages, writes, |path| read(&root.join(path)))?;
    let mut written = Vec::new();
    for (path, before, after) in changes {
        if before != after {
            write(&root.join(&path), &after)?;
            written.push(path);
        }
    }
    Ok(written)
}

/// Every group's declared version projected onto its members, from the file
/// as it stands: what `generate versions` runs for a hand-edited file, a
/// crate newly joining a group, or a scratch tree. Returns the paths it
/// changed — none on a tree that already agrees.
pub fn project_file(metadata: &Metadata, groups: &GroupFile) -> Result<Vec<String>> {
    let writes = groups
        .group
        .keys()
        .map(|name| Ok((name.clone(), groups.version_of(name)?)))
        .collect::<Result<Vec<_>>>()?;
    project(metadata, groups, &writes)
}

/// The version `group`'s `tag` names: the tag less the group's prefix.
fn tag_version(groups: &GroupFile, group: &str, tag: &str) -> Result<Version> {
    let template = &groups
        .group
        .get(group)
        .with_context(|| format!("no group named `{group}`"))?
        .tag;
    let version = tag
        .strip_prefix(tag_prefix(template)?)
        .with_context(|| format!("tag `{tag}` is not `{template}`"))?;
    Version::parse(version)
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))
}

fn write(path: &Path, text: &str) -> Result<()> {
    fs::write(path, text).with_context(|| format!("Failed to write {}", path.display()))
}

/// Rewrites the `[package]` version of `manifest`: [`set_version`].
fn set_package_version(manifest: &str, version: &Version) -> Result<String> {
    set_version(manifest, &["package"], version)
}

/// Rewrites `group`'s `version` in the group file: [`set_version`].
fn set_group_version(file: &str, group: &str, version: &Version) -> Result<String> {
    set_version(file, &["group", group], version)
}

/// Rewrites the `version` value under `table` in the TOML `text`, as a
/// document rather than a reformat — the one way `pin_workspace` edits the
/// root manifest: the key keeps its indent and the alignment tombi gave it,
/// and every other byte stays. Refuses a text without that value; a crate
/// inheriting its version declares a table there, not a value.
fn set_version(text: &str, table: &[&str], version: &Version) -> Result<String> {
    let path = table.join(".");
    let mut document: DocumentMut = text
        .parse()
        .with_context(|| format!("the text declaring `[{path}]` does not parse"))?;
    let mut item = document.as_item_mut();
    for key in table {
        item = item
            .get_mut(key)
            .with_context(|| format!("no `[{path}]` to rewrite a version in"))?;
    }
    let value = item
        .get_mut("version")
        .and_then(Item::as_value_mut)
        .with_context(|| format!("no `version` value to rewrite in `[{path}]`"))?;
    let decor = value.decor().clone();
    *value = Value::from(version.to_string());
    *value.decor_mut() = decor;
    Ok(document.to_string())
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

/// A withdrawal in flight: a bump's write landed and the release built from
/// it was withdrawn, so a branch declares a version no tag names until the
/// revert `release undo` leaves lands. Either a series whose declared version
/// has no tag, or `main` targeting more than one minor past the newest series
/// — or, with none open, the last stable release — which is what a cut whose
/// series is gone leaves behind. Every bump refuses while one is in flight.
#[derive(Debug, PartialEq, Eq)]
pub struct InFlight {
    /// The branch carrying the write: a series, or `main`.
    pub branch: String,
    /// The version it declares.
    pub version: Version,
}

impl InFlight {
    /// The branch the revert lands from, which is how its open pull request
    /// is found.
    pub fn revert_head(&self) -> String {
        format!(
            "{}{}",
            crate::constants::RELEASE_UNDO_HEAD_PREFIX,
            self.branch
        )
    }
}

/// Reads a withdrawal in flight off the remote's refs: `series` as `(branch,
/// declared app version)`, `tags` the published `v*` tags, `main` the version
/// `main` declares, and `last_stable` the newest stable tag.
pub fn withdrawal_in_flight(
    series: &[(String, Version)],
    tags: &[String],
    main: &Version,
    last_stable: Option<&Version>,
) -> Result<Option<InFlight>> {
    for (branch, declared) in series {
        if !declared.is_nightly() && !tags.iter().any(|tag| *tag == declared.tag()) {
            return Ok(Some(InFlight {
                branch: branch.clone(),
                version: declared.clone(),
            }));
        }
    }
    // After a cut, `main` targets one minor past the series it opened.
    let newest = series
        .iter()
        .map(|(_, declared)| declared.target())
        .max()
        .or_else(|| last_stable.map(Version::target));
    let Some(newest) = newest else {
        return Ok(None);
    };
    if main.target() > newest.next_main()?.target() {
        return Ok(Some(InFlight {
            branch: MAIN.to_owned(),
            version: main.clone(),
        }));
    }
    Ok(None)
}

/// The open series as the remote has them: every `v*.x` branch with the app
/// version its group file declares, read without a checkout.
pub fn open_series_from_remote() -> Result<Vec<(String, Version)>> {
    git::run(&[
        "fetch",
        "origin",
        "+refs/heads/v*.x:refs/remotes/origin/v*.x",
    ])?;
    let mut series = Vec::new();
    for branch in git::remote_branches_like("v*.x")? {
        let file = git::show(&format!("origin/{branch}"), GROUP_FILE)?;
        series.push((branch, GroupFile::parse(&file)?.app_version()?));
    }
    Ok(series)
}

/// The open pull request reverting `in_flight`'s write, if it exists yet.
pub fn open_revert(in_flight: &InFlight) -> Result<Option<u64>> {
    github::open_pull_request_from(&in_flight.revert_head())
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

    /// Scenario _A withdrawal in flight_ and _A cut's write without its
    /// series_: a series declaring a version no tag names, or `main` targeting
    /// past the series a cut should have opened, each naming the revert's
    /// branch; a tagged series under a `main` one minor ahead is the normal
    /// state.
    #[test]
    fn a_withdrawal_in_flight_is_read_off_the_refs() {
        let series = [("v0.2.x".to_owned(), v("0.2.0-pre.2"))];
        let stable = v("0.1.0");
        let tagged = msgs(&["v0.1.0", "v0.2.0-pre.1", "v0.2.0-pre.2"]);
        let main = v("0.3.0-nightly");
        assert_eq!(
            withdrawal_in_flight(&series, &tagged, &main, Some(&stable)).unwrap(),
            None
        );

        let untagged = msgs(&["v0.1.0", "v0.2.0-pre.1"]);
        let found = withdrawal_in_flight(&series, &untagged, &main, Some(&stable))
            .unwrap()
            .unwrap();
        assert_eq!(found.branch, "v0.2.x");
        assert_eq!(found.version, v("0.2.0-pre.2"));
        assert_eq!(found.revert_head(), "botswain/release-undo/v0.2.x");

        let past = v("0.4.0-nightly");
        let found = withdrawal_in_flight(&series, &tagged, &past, Some(&stable))
            .unwrap()
            .unwrap();
        assert_eq!(found.branch, "main");
        assert_eq!(found.version, past);
        assert_eq!(found.revert_head(), "botswain/release-undo/main");

        // With no series open, the last stable release is the mark.
        let only_stable = msgs(&["v0.1.0"]);
        assert_eq!(
            withdrawal_in_flight(&[], &only_stable, &v("0.2.0-nightly"), Some(&stable)).unwrap(),
            None
        );
        assert!(
            withdrawal_in_flight(&[], &only_stable, &v("0.3.0-nightly"), Some(&stable))
                .unwrap()
                .is_some()
        );
        // Before any release there is nothing to be in flight.
        assert_eq!(
            withdrawal_in_flight(&[], &[], &v("0.1.0-nightly"), None).unwrap(),
            None
        );
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

    /// A commit on a library's own paths, which counts for the library alone.
    fn library_commit(group: &str, subject: &str) -> history::Commit {
        commit_on(
            &format!("crates/{group}/src/lib.rs"),
            &[group],
            subject,
            false,
        )
    }

    /// Scenarios _Library-only cycle_, _Before the first stable_, and the
    /// refusal's text with nothing moved: the app's refusal names what the
    /// cut is measured from and the libraries a group-scoped cut would mint.
    #[test]
    fn a_cut_with_nothing_counted_for_the_app_says_what_moved() {
        let bedrock = group_plan("bedrock", Some("0.1.1"), Some("bedrock-v0.1.0"), 2);
        let outpost = group_plan("outpost", None, Some("outpost-v0.1.0"), 0);
        let planned = [bedrock, outpost];
        let refusal = app_unmoved("waypoints", Some(&v("0.1.0")), &planned);
        assert_eq!(
            refusal,
            "nothing to cut for `waypoints`: no counted commit since `v0.1.0`; `bedrock` moved, and `release plan cut --group <name>` mints one alone"
        );
        let first = app_unmoved("waypoints", None, &planned);
        assert!(first.contains("in its history"), "{first}");
        let nothing = app_unmoved("waypoints", Some(&v("0.1.0")), &[]);
        assert!(nothing.ends_with("; no library moved either"), "{nothing}");
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
            &[],
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

        let error = cut_target(&v("1.2.0"), &commits, &groups, false, &[]).unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains("abc1234de fix: tighten the envelope"),
            "{text}"
        );
        assert!(
            text.contains("--major") && text.contains("2.0.0-pre.1"),
            "{text}"
        );

        let target = cut_target(&v("1.2.0"), &commits, &groups, true, &[]).unwrap();
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
            &[],
        )
        .unwrap();
        assert_eq!(target, v("2.0.0-pre.1"));
        assert_eq!(v("0.1.0").cut_major().unwrap(), v("1.0.0-pre.1"));
    }

    fn change(
        status: crate::version_integrity::Status,
        path: &str,
    ) -> crate::version_integrity::Change {
        crate::version_integrity::Change {
            status,
            path: path.to_owned(),
        }
    }

    /// Scenario _Domain fixture modified_: a frozen fixture modified since
    /// the last stable release breaks its group — a minor under right-shift,
    /// from fixes alone — and the app, whose cut refuses without `--major`,
    /// naming the fixture. An added fixture breaks nothing.
    #[test]
    fn a_modified_fixture_breaks_its_group_and_the_app() {
        use crate::version_integrity::Status;
        let groups = workspace_groups();
        let changes = [
            change(
                Status::Modified,
                "covenant/fixtures/domain/v1/envelope.json",
            ),
            change(Status::Added, "covenant/fixtures/bedrock/new.json"),
            change(Status::Modified, "crates/domain/src/lib.rs"),
        ];
        let broken = fixture_breaks(&changes, &groups);
        assert_eq!(broken.keys().collect::<Vec<_>>(), ["domain"]);
        assert_eq!(
            broken["domain"],
            ["covenant/fixtures/domain/v1/envelope.json"]
        );

        let fixes = [library_commit("domain", "fix(domain): a bug")];
        let domain = GroupPlan::decide(
            "domain",
            "domain-v{version}",
            &v("0.1.0"),
            &tags(&["domain-v0.1.0"]),
            &fixes,
            &groups,
            Raise {
                fixtures: true,
                ..Raise::default()
            },
        )
        .unwrap();
        assert_eq!(domain.next.as_deref(), Some("0.2.0"));
        assert!(domain.fixture_break);
        assert!(
            domain.describe().contains("a frozen fixture changed"),
            "{}",
            domain.describe()
        );

        let changed: Vec<String> = broken.values().flatten().cloned().collect();
        let commits = [app_commit("fix: a bug", false)];
        let error = cut_target(&v("1.2.0"), &commits, &groups, false, &changed).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("`covenant/fixtures/domain/v1/envelope.json`, a frozen fixture, changed"),
            "{error}"
        );
        assert_eq!(
            cut_target(&v("1.2.0"), &commits, &groups, true, &changed).unwrap(),
            v("2.0.0-pre.1")
        );
    }

    /// Scenario _A group's README is edited_: the directory's own README is
    /// not a fixture and raises nothing; a README any deeper is data.
    #[test]
    fn a_group_s_readme_raises_no_floor() {
        use crate::version_integrity::Status;
        let groups = workspace_groups();
        let readme = [
            change(Status::Modified, "covenant/fixtures/domain/README.md"),
            change(Status::Deleted, "covenant/fixtures/domain/README.md"),
        ];
        assert!(fixture_breaks(&readme, &groups).is_empty());

        let deeper = [change(
            Status::Deleted,
            "covenant/fixtures/domain/corpus/README.md",
        )];
        assert_eq!(
            fixture_breaks(&deeper, &groups).keys().collect::<Vec<_>>(),
            ["domain"]
        );
    }

    fn decide(
        group: &str,
        current: &str,
        tags: &[&str],
        commits: &[history::Commit],
        floor: Option<Floor>,
    ) -> GroupPlan {
        decide_measured(group, current, tags, commits, floor, None)
    }

    /// [`decide`] with what the API check `detected` against the last tag.
    fn decide_measured(
        group: &str,
        current: &str,
        tags: &[&str],
        commits: &[history::Commit],
        floor: Option<Floor>,
        detected: Option<Required>,
    ) -> GroupPlan {
        let tags: Vec<String> = tags.iter().map(|tag| (*tag).to_owned()).collect();
        GroupPlan::decide(
            group,
            &format!("{group}-v{{version}}"),
            &v(current),
            &tags,
            commits,
            &workspace_groups(),
            Raise {
                floor,
                detected,
                fixtures: false,
            },
        )
        .unwrap()
    }

    /// Scenario _Undeclared API break reaches a cut_: fixes alone would move
    /// `outpost` a patch, and the break the check found against its tag
    /// raises it to breaking — a minor, under right-shift.
    #[test]
    fn an_undeclared_api_raises_a_cut_to_breaking() {
        let fixes = [library_commit("outpost", "fix(outpost): a bug")];
        let broke = decide_measured(
            "outpost",
            "0.2.0",
            &["outpost-v0.2.0"],
            &fixes,
            None,
            Some(Required::Major),
        );
        assert_eq!(broke.next.as_deref(), Some("0.3.0"));
        assert_eq!(broke.api, Some(Required::Major));
        assert!(
            broke.describe().contains("the API check found a break"),
            "{}",
            broke.describe()
        );

        let clean = decide_measured(
            "outpost",
            "0.2.0",
            &["outpost-v0.2.0"],
            &fixes,
            None,
            Some(Required::Patch),
        );
        assert_eq!(clean.next.as_deref(), Some("0.2.1"));
        assert!(
            clean.describe().contains("found no break"),
            "{}",
            clean.describe()
        );

        // A surface that grew is a feature, which right-shifts to the same
        // patch the fixes move before `1.0`, and to a minor after it; the
        // line says what was found either way.
        let grew = decide_measured(
            "outpost",
            "0.2.0",
            &["outpost-v0.2.0"],
            &fixes,
            None,
            Some(Required::Minor),
        );
        assert_eq!(grew.next.as_deref(), Some("0.2.1"));
        assert!(
            grew.describe()
                .contains("the API check found the surface grew"),
            "{}",
            grew.describe()
        );
        let grew_after_one = decide_measured(
            "outpost",
            "1.4.2",
            &["outpost-v1.4.2"],
            &fixes,
            None,
            Some(Required::Minor),
        );
        assert_eq!(grew_after_one.next.as_deref(), Some("1.5.0"));

        // After `1.0` the same evidence is a major.
        let major = decide_measured(
            "outpost",
            "1.4.2",
            &["outpost-v1.4.2"],
            &fixes,
            None,
            Some(Required::Major),
        );
        assert_eq!(major.next.as_deref(), Some("2.0.0"));
    }

    /// Scenario _Declared break with no evidence_: the declaration alone
    /// breaks the group — behaviour changes and macro output escape the
    /// checker, and the header is trusted.
    #[test]
    fn a_declared_break_with_no_evidence_still_breaks() {
        let declared = [library_commit(
            "bedrock",
            "fix(bedrock)!: tighten the query",
        )];
        let bedrock = decide_measured(
            "bedrock",
            "0.3.0",
            &["bedrock-v0.3.0"],
            &declared,
            None,
            Some(Required::Patch),
        );
        assert_eq!(bedrock.next.as_deref(), Some("0.4.0"));
    }

    /// Scenario _Group with no tag yet_: nothing is measured — there is no
    /// baseline — and the line says the cut mints one; the bump comes from
    /// the declarations alone.
    #[test]
    fn a_group_with_no_tag_is_not_measured_and_says_so() {
        let feature = [library_commit("transload", "feat(transload): a thing")];
        let transload = decide_measured("transload", "0.1.0", &[], &feature, None, None);
        assert_eq!(transload.api, None);
        assert_eq!(transload.next.as_deref(), Some("0.1.1"));
        let line = transload.describe();
        assert!(
            line.contains("no tag to measure an API break against until this cut mints one"),
            "{line}"
        );
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

    /// Scenario _First cut of a group with no tag_: the file's declaration is the
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
            "tooling/release-groups.toml,crates/bedrock/Cargo.toml,crates/bedrock-core/Cargo.toml,crates/bedrock-derive/Cargo.toml,Cargo.lock"
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
        let manifest = indoc::indoc! {r#"
            [package]
              name              = "cartograph"
              version           = "0.1.0"
              edition.workspace = true
        "#};
        assert_eq!(
            set_package_version(manifest, &v("0.2.0-pre.1")).unwrap(),
            indoc::indoc! {r#"
                [package]
                  name              = "cartograph"
                  version           = "0.2.0-pre.1"
                  edition.workspace = true
            "#}
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
    /// transition has to write all of it in one commit — and the series a
    /// cut opens takes its pins in that same commit, so the root manifest
    /// rides along there and nowhere else.
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
            let expected = if transition == Transition::Cut {
                "tooling/release-groups.toml,src-tauri/Cargo.toml,crates/cartograph/Cargo.toml,Cargo.toml,Cargo.lock"
            } else {
                "tooling/release-groups.toml,src-tauri/Cargo.toml,crates/cartograph/Cargo.toml,Cargo.lock"
            };
            assert_eq!(landing.paths, expected, "{transition:?}");
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
        // `main` keeps its path dependencies: the pins are the series' alone.
        assert_eq!(
            main.paths,
            "tooling/release-groups.toml,src-tauri/Cargo.toml,crates/cartograph/Cargo.toml,Cargo.lock"
        );

        let advance = plan(Transition::Advance, "0.2.0-pre.1", &[]);
        assert_eq!(advance.main_landing(&groups).unwrap(), None);
    }

    fn pins(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(group, tag)| ((*group).to_owned(), (*tag).to_owned()))
            .collect()
    }

    fn group_plan(
        group: &str,
        next: Option<&str>,
        last_tag: Option<&str>,
        counted: usize,
    ) -> GroupPlan {
        GroupPlan {
            group: group.to_owned(),
            current: "0.1.0".to_owned(),
            next: next.map(str::to_owned),
            tag: next.map(|next| format!("{group}-v{next}")),
            last_tag: last_tag.map(str::to_owned),
            counted,
            api: None,
            fixture_break: false,
        }
    }

    /// Scenario _A cut starts pinned to what it minted_: a group the cut
    /// mints is pinned at that tag, one it leaves alone at its last tag, and
    /// one that has never been tagged is not pinned at all.
    #[test]
    fn a_cut_pins_what_it_mints_and_the_last_tag_of_what_it_leaves() {
        let plan = plan(Transition::Cut, "0.1.0", &["feat: charts"]).with_groups(vec![
            group_plan("bedrock", Some("0.2.0"), Some("bedrock-v0.1.0"), 2),
            group_plan("domain", None, Some("domain-v0.1.0"), 0),
            group_plan("transload", None, None, 0),
        ]);
        assert_eq!(
            plan.pins(),
            pins(&[("bedrock", "bedrock-v0.2.0"), ("domain", "domain-v0.1.0")])
        );
        assert!(plan.opens_series());
        assert!(!Plan::library(group_plan("bedrock", Some("0.2.0"), None, 1)).opens_series());
    }

    const SERIES_GROUPS: &str = r#"
        [bookkeeping]
        paths = [ "nix/releases/" ]
        bot-subjects = [ "chore(release):" ]

        [group.bedrock]
        version = "0.1.0"
        members = [ "crates/bedrock", "crates/bedrock-core" ]
        tag = "bedrock-v{version}"
        fixtures = "covenant/fixtures/bedrock/"

        [group.outpost]
        version = "0.1.0"
        members = [ "crates/outpost" ]
        internals = [ "crates/outpost-protocol" ]
        tag = "outpost-v{version}"
        fixtures = "covenant/fixtures/outpost/"

        [group.waypoints]
        version = "0.1.0-nightly"
        members = [ "src-tauri" ]
        internals = [ "crates/store" ]
        tag = "v{version}"
        fixtures = "covenant/fixtures/waypoints/"
    "#;

    /// A root manifest as `main` carries it, aligned as tombi leaves it.
    const IN_TREE: &str = indoc::indoc! {r#"
        #:tombi toml-version = "v1.0.0"

        [workspace]
          resolver = "3"
          members  = [
            "crates/bedrock",
            "crates/bedrock-core",
            "crates/outpost",
            "crates/outpost-protocol",
            "crates/store",
            "src-tauri",
            "tooling/xtask",
          ]

          [workspace.dependencies]
            bedrock          = { path = "crates/bedrock" }
            bedrock-core     = { path = "crates/bedrock-core" }
            outpost          = { path = "crates/outpost", default-features = false }
            outpost-protocol = { path = "crates/outpost-protocol" }
            store            = { path = "crates/store" }

            serde = "1"
    "#};

    /// Scenario _A cut starts pinned to what it minted_, in the manifest: the
    /// minted group's crates take its tag and leave the members under the
    /// comment, a group with no tag stays in-tree, and nothing else moves —
    /// the keys keep tombi's alignment, and a pinned entry stays on its line.
    #[test]
    fn pinning_rewrites_the_pinned_group_s_entries_and_nothing_else() {
        let groups = GroupFile::parse(SERIES_GROUPS).unwrap();
        let pinned =
            pin_workspace(IN_TREE, &pins(&[("bedrock", "bedrock-v0.2.0")]), &groups).unwrap();
        let expected = indoc::indoc! {r#"
            #:tombi toml-version = "v1.0.0"

            [workspace]
              resolver = "3"
              # On a series the library groups are pinned, below, to the tags the cut minted.
              # Their crates stay on disk so a cherry-pick from main applies, and build
              # nothing: a library change reaches this series only by a pin that `patch`
              # moves (docs/CONTRIBUTING.md).
              members  = [
                "crates/outpost",
                "crates/outpost-protocol",
                "crates/store",
                "src-tauri",
                "tooling/xtask",
              ]

              [workspace.dependencies]
                bedrock          = { git = "https://github.com/jmuchovej/waypoints", tag = "bedrock-v0.2.0" }
                bedrock-core     = { git = "https://github.com/jmuchovej/waypoints", tag = "bedrock-v0.2.0" }
                outpost          = { path = "crates/outpost", default-features = false }
                outpost-protocol = { path = "crates/outpost-protocol" }
                store            = { path = "crates/store" }

                serde = "1"
        "#};
        assert_eq!(pinned, expected);
    }

    /// Scenario _One copy of each group per binary_, from the writer's side:
    /// every crate of a group — internals included — pins the group's one
    /// tag, a dependency's other keys survive the pin, and the pull-request
    /// check reads back exactly what was written.
    #[test]
    fn every_crate_of_a_group_pins_the_group_s_one_tag() {
        let groups = GroupFile::parse(SERIES_GROUPS).unwrap();
        let pinned = pin_workspace(
            IN_TREE,
            &pins(&[("bedrock", "bedrock-v0.2.0"), ("outpost", "outpost-v0.3.0")]),
            &groups,
        )
        .unwrap();
        assert!(
            pinned.contains(
                "    outpost          = { git = \"https://github.com/jmuchovej/waypoints\", tag = \"outpost-v0.3.0\", default-features = false }\n"
            ),
            "{pinned}"
        );
        assert!(
            pinned.contains(
                "    outpost-protocol = { git = \"https://github.com/jmuchovej/waypoints\", tag = \"outpost-v0.3.0\" }\n"
            ),
            "{pinned}"
        );
        assert!(
            pinned.contains(
                "  members  = [\n    \"crates/store\",\n    \"src-tauri\",\n    \"tooling/xtask\",\n  ]\n"
            ),
            "{pinned}"
        );

        let read = crate::version_integrity::pins_in(ROOT_MANIFEST, &pinned, &groups).unwrap();
        let read: Vec<(&str, &str)> = read
            .iter()
            .map(|pin| (pin.group.as_str(), pin.tag.as_str()))
            .collect();
        assert_eq!(
            read,
            [
                ("bedrock", "bedrock-v0.2.0"),
                ("bedrock", "bedrock-v0.2.0"),
                ("outpost", "outpost-v0.3.0"),
                ("outpost", "outpost-v0.3.0"),
            ]
        );
    }

    /// This tree's manifest, pinned: every library crate leaves the workspace
    /// and takes its group's tag, the app's crates and the tooling stay on
    /// their paths, and the result is still the strict TOML nix reads.
    #[test]
    fn pinning_this_tree_leaves_only_the_app_and_the_tooling_in_the_workspace() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest = fs::read_to_string(root.join(ROOT_MANIFEST)).unwrap();
        let groups = workspace_groups();
        let tags = pins(&[
            ("bedrock", "bedrock-v0.2.0"),
            ("domain", "domain-v0.2.0"),
            ("outpost", "outpost-v0.2.0"),
            ("transload", "transload-v0.1.1"),
        ]);
        let pinned = pin_workspace(&manifest, &tags, &groups).unwrap();

        let table: toml::Table = pinned.parse().unwrap();
        let members: Vec<&str> = table["workspace"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(toml::Value::as_str)
            .collect();
        assert_eq!(
            members,
            [
                "crates/cartograph",
                "crates/store",
                "src-tauri",
                "tooling/gh-workflow",
                "tooling/xtask",
            ]
        );
        let dependencies = table["workspace"]["dependencies"].as_table().unwrap();
        for name in ["cartograph", "store", "gh-workflow"] {
            assert!(dependencies[name].get("path").is_some(), "{name}");
            assert!(dependencies[name].get("git").is_none(), "{name}");
        }

        let read = crate::version_integrity::pins_in(ROOT_MANIFEST, &pinned, &groups).unwrap();
        assert_eq!(read.len(), 9, "{read:?}");
        for pin in &read {
            assert_eq!(tags[&pin.group], pin.tag, "{pin:?}");
        }
    }

    fn tags(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    /// The tag a pin moves to is the newest on its line, as cargo reads a
    /// caret: before `1.0` the minor is the line, after it the major; the
    /// pin itself and anything older never qualify, nor does another group.
    #[test]
    fn a_pin_moves_to_the_newest_tag_on_its_line() {
        let bedrock = tags(&[
            "bedrock-v0.1.9",
            "bedrock-v0.2.0",
            "bedrock-v0.2.1",
            "bedrock-v0.2.3",
            "bedrock-v0.3.0",
            "outpost-v0.2.9",
        ]);
        assert_eq!(
            newer_on_line("bedrock-v", &v("0.2.0"), &bedrock),
            Some(v("0.2.3"))
        );
        assert_eq!(newer_on_line("bedrock-v", &v("0.2.3"), &bedrock), None);
        assert_eq!(newer_on_line("bedrock-v", &v("0.3.0"), &bedrock), None);

        let outpost = tags(&["outpost-v1.4.2", "outpost-v1.5.0", "outpost-v2.0.0"]);
        assert_eq!(
            newer_on_line("outpost-v", &v("1.4.2"), &outpost),
            Some(v("1.5.0"))
        );
        assert_eq!(
            newer_on_line("outpost-v", &v("0.0.3"), &tags(&["outpost-v0.0.4"])),
            None
        );
    }

    /// Scenario _A fix reaches a series by pin_: `main` minted
    /// `bedrock-v0.2.1` on an unbroken line, the series' pin moves to it,
    /// and the patch writes that move beside the app's bump — and nothing on
    /// `main`.
    #[test]
    fn a_fix_reaches_a_series_by_pin() {
        use crate::releases::index::fixture::{GROUPS, Repo};
        let groups = GroupFile::parse(GROUPS).unwrap();
        let repo = Repo::new();
        repo.commit("crates/bedrock/src/lib.rs", "feat(bedrock): the library");
        repo.tag("bedrock-v0.2.0");
        repo.commit("crates/bedrock/src/lib.rs", "fix(bedrock): a bug");
        repo.tag("bedrock-v0.2.1");

        let minted = tags(&["bedrock-v0.2.0", "bedrock-v0.2.1"]);
        let to =
            newer_on_line("bedrock-v", &v("0.2.0"), &minted).map(|to| format!("bedrock-v{to}"));
        assert_eq!(to.as_deref(), Some("bedrock-v0.2.1"));
        let commits = repo.walk("bedrock-v0.2.1..HEAD");
        let pin = PinMove::decide("bedrock", "bedrock-v0.2.0", to, &commits, &groups);
        assert!(pin.moves());
        assert_eq!(pin.target(), "bedrock-v0.2.1");
        assert!(pin.breaking.is_empty(), "{pin:?}");
        assert_eq!(
            pin.describe(),
            "`bedrock`: `bedrock-v0.2.0` → `bedrock-v0.2.1`"
        );

        let groups = workspace_groups();
        let patch = plan(Transition::Patch, "0.2.0", &[]).with_moves(vec![pin]);
        assert!(patch.moves_a_pin());
        assert_eq!(patch.pins(), pins(&[("bedrock", "bedrock-v0.2.1")]));
        assert_eq!(
            patch.landing(&groups).unwrap().unwrap().paths,
            "tooling/release-groups.toml,src-tauri/Cargo.toml,crates/cartograph/Cargo.toml,Cargo.toml,Cargo.lock"
        );
        assert_eq!(patch.main_landing(&groups).unwrap(), None);
    }

    /// Scenario _A line broken on main cannot be patched into a series_: the
    /// pin stays, the breaking commit is named, a new cut is pointed at, and
    /// the patch still moves the app.
    #[test]
    fn a_line_broken_on_main_cannot_be_patched_into_a_series() {
        use crate::releases::index::fixture::{GROUPS, Repo};
        let groups = GroupFile::parse(GROUPS).unwrap();
        let repo = Repo::new();
        repo.commit("crates/bedrock/src/lib.rs", "feat(bedrock): the library");
        repo.tag("bedrock-v0.2.0");
        repo.commit(
            "crates/bedrock/src/lib.rs",
            "feat(bedrock)!: rename the filter API",
        );
        let sha = repo.head();
        repo.commit(
            "crates/bedrock/src/lib.rs",
            "fix(bedrock): the fix a series wants",
        );
        repo.tag("bedrock-v0.3.0");

        let minted = tags(&["bedrock-v0.2.0", "bedrock-v0.3.0"]);
        assert_eq!(newer_on_line("bedrock-v", &v("0.2.0"), &minted), None);
        let commits = repo.walk("bedrock-v0.2.0..HEAD");
        let pin = PinMove::decide("bedrock", "bedrock-v0.2.0", None, &commits, &groups);
        assert!(!pin.moves());
        assert_eq!(
            pin.breaking,
            [format!(
                "{} feat(bedrock)!: rename the filter API",
                &sha[..9]
            )]
        );
        let line = pin.describe();
        assert!(line.contains("stays at `bedrock-v0.2.0`"), "{line}");
        assert!(line.contains(&sha[..9]), "{line}");
        assert!(
            line.contains("a new cut is the way to ship a fix"),
            "{line}"
        );

        let groups = workspace_groups();
        let patch = plan(Transition::Patch, "0.2.0", &[]).with_moves(vec![pin]);
        assert!(!patch.moves_a_pin());
        assert!(patch.pins().is_empty());
        assert_eq!(
            patch.landing(&groups).unwrap().unwrap().paths,
            "tooling/release-groups.toml,src-tauri/Cargo.toml,crates/cartograph/Cargo.toml,Cargo.lock"
        );
    }

    /// A break after the tag the pin moves to is named too: the move is
    /// sound, and a fix behind the break is a new cut's to ship.
    #[test]
    fn a_break_after_the_moved_tag_is_named() {
        use crate::releases::index::fixture::{GROUPS, Repo};
        let groups = GroupFile::parse(GROUPS).unwrap();
        let repo = Repo::new();
        repo.commit("crates/bedrock/src/lib.rs", "fix(bedrock): a bug");
        repo.tag("bedrock-v0.2.1");
        repo.commit("crates/bedrock/src/lib.rs", "feat(bedrock)!: a break");

        let commits = repo.walk("bedrock-v0.2.1..HEAD");
        let pin = PinMove::decide(
            "bedrock",
            "bedrock-v0.2.0",
            Some("bedrock-v0.2.1".to_owned()),
            &commits,
            &groups,
        );
        assert!(pin.moves());
        let line = pin.describe();
        assert!(
            line.starts_with(
                "`bedrock`: `bedrock-v0.2.0` → `bedrock-v0.2.1`; main broke the line after it ("
            ),
            "{line}"
        );
        assert!(
            line.ends_with("), so a later fix needs a new cut"),
            "{line}"
        );
    }

    /// A pinned manifest takes a moved pin in place: the tag alone changes,
    /// the members and the comment stay as they are, and a group that does
    /// not move keeps its tag.
    #[test]
    fn a_moved_pin_changes_only_its_tag() {
        let groups = GroupFile::parse(SERIES_GROUPS).unwrap();
        let pinned = pin_workspace(
            IN_TREE,
            &pins(&[("bedrock", "bedrock-v0.2.0"), ("outpost", "outpost-v0.3.0")]),
            &groups,
        )
        .unwrap();
        let moved =
            pin_workspace(&pinned, &pins(&[("bedrock", "bedrock-v0.2.1")]), &groups).unwrap();
        assert_eq!(moved, pinned.replace("bedrock-v0.2.0", "bedrock-v0.2.1"));
        assert!(moved.contains("tag = \"outpost-v0.3.0\""), "{moved}");
        assert_eq!(moved.matches("# On a series").count(), 1, "{moved}");
    }

    /// A group file for [`SERIES_GROUPS`]' groups, aligned as tombi leaves
    /// ours — the app's table wider than the libraries', as in the real one.
    fn series_file(bedrock: &str, outpost: &str, app: &str) -> String {
        formatdoc! { r#"
            [group.bedrock]
              version  = "{bedrock}"
              members  = [ "crates/bedrock", "crates/bedrock-core" ]
              tag      = "bedrock-v{{version}}"

            [group.outpost]
              version  = "{outpost}"
              members  = [ "crates/outpost" ]
              tag      = "outpost-v{{version}}"

            [group.waypoints]
              version                         = "{app}"
              members                         = [ "src-tauri" ]
              tag                             = "v{{version}}"
        "# }
    }

    /// The file's line edit: the named group's `version` alone, at its own
    /// alignment, and a refusal for a group the file does not declare.
    #[test]
    fn a_group_s_version_line_moves_alone() {
        let file = series_file("0.1.0", "0.1.0", "0.2.0");
        assert_eq!(
            set_group_version(&file, "outpost", &v("0.3.0")).unwrap(),
            series_file("0.1.0", "0.3.0", "0.2.0")
        );
        assert_eq!(
            set_group_version(&file, "waypoints", &v("0.2.1")).unwrap(),
            series_file("0.1.0", "0.1.0", "0.2.1")
        );
        assert!(set_group_version(&file, "domain", &v("0.1.0")).is_err());
    }

    fn manifest(name: &str, version: &str) -> String {
        formatdoc! { r#"
            [package]
              name    = "{name}"
              version = "{version}"
        "# }
    }

    /// Scenarios _A patch writes a moved pin's version_ and _A held pin
    /// leaves the file alone_, and what a dry patch renders: on a series
    /// pinning `bedrock` and `outpost`, the app's write reaches its manifest
    /// and lockfile entry, the moved pin's write is `bedrock`'s line in the
    /// file alone — its crates are out of the workspace, and their lockfile
    /// entries are cargo's — and `outpost`, not written, keeps its line.
    #[test]
    fn a_pinned_group_projects_onto_the_file_alone() {
        let groups = GroupFile::parse(SERIES_GROUPS).unwrap();
        let root_manifest = pin_workspace(
            IN_TREE,
            &pins(&[("bedrock", "bedrock-v0.2.0"), ("outpost", "outpost-v0.3.0")]),
            &groups,
        )
        .unwrap();
        let file = series_file("0.2.0", "0.3.0", "0.2.0");
        let lock = indoc::indoc! {r#"
            [[package]]
            name = "Waypoints"
            version = "0.2.0"

            [[package]]
            name = "bedrock"
            version = "0.2.0"
            source = "git+https://github.com/jmuchovej/waypoints?tag=bedrock-v0.2.0#abc"
        "#};
        let packages = vec![("Waypoints".to_owned(), "src-tauri".to_owned())];
        let read = |path: &str| -> Result<String> {
            Ok(match path {
                ROOT_MANIFEST => root_manifest.clone(),
                GROUP_FILE => file.clone(),
                LOCKFILE => lock.to_owned(),
                "src-tauri/Cargo.toml" => manifest("Waypoints", "0.2.0"),
                other => bail!("a pinned crate's manifest is never read: {other}"),
            })
        };
        let writes = [
            ("waypoints".to_owned(), v("0.2.1")),
            ("bedrock".to_owned(), v("0.2.1")),
        ];

        let changes = projection(&groups, &packages, &writes, read).unwrap();
        let paths: Vec<&str> = changes.iter().map(|(path, _, _)| path.as_str()).collect();
        assert_eq!(paths, [GROUP_FILE, "src-tauri/Cargo.toml", LOCKFILE]);
        assert_eq!(changes[0].2, series_file("0.2.1", "0.3.0", "0.2.1"));
        assert_eq!(changes[1].2, manifest("Waypoints", "0.2.1"));
        assert_eq!(changes[2].2, lock.replacen("0.2.0", "0.2.1", 1));
    }

    /// Scenario _A dry cut renders the file_, as far as the projection goes:
    /// on `main`, where nothing is pinned, the minted group's write reaches
    /// every member manifest and lockfile entry beside its line in the file,
    /// and the app's bump the same; the file comes first and the lockfile
    /// last, each carrying both writes.
    #[test]
    fn an_in_tree_group_projects_onto_every_member() {
        let groups = GroupFile::parse(SERIES_GROUPS).unwrap();
        let file = series_file("0.1.0", "0.1.0", "0.1.0-nightly");
        let lock = indoc::indoc! {r#"
            [[package]]
            name = "Waypoints"
            version = "0.1.0-nightly"

            [[package]]
            name = "bedrock"
            version = "0.1.0"

            [[package]]
            name = "bedrock-core"
            version = "0.1.0"
        "#};
        let packages = vec![
            ("Waypoints".to_owned(), "src-tauri".to_owned()),
            ("bedrock".to_owned(), "crates/bedrock".to_owned()),
            ("bedrock-core".to_owned(), "crates/bedrock-core".to_owned()),
        ];
        let read = |path: &str| -> Result<String> {
            Ok(match path {
                ROOT_MANIFEST => IN_TREE.to_owned(),
                GROUP_FILE => file.clone(),
                LOCKFILE => lock.to_owned(),
                "src-tauri/Cargo.toml" => manifest("Waypoints", "0.1.0-nightly"),
                "crates/bedrock/Cargo.toml" => manifest("bedrock", "0.1.0"),
                "crates/bedrock-core/Cargo.toml" => manifest("bedrock-core", "0.1.0"),
                other => bail!("unexpected read of {other}"),
            })
        };
        let writes = [
            ("waypoints".to_owned(), v("0.2.0-nightly")),
            ("bedrock".to_owned(), v("0.2.0")),
        ];

        let changes = projection(&groups, &packages, &writes, read).unwrap();
        let paths: Vec<&str> = changes.iter().map(|(path, _, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                GROUP_FILE,
                "src-tauri/Cargo.toml",
                "crates/bedrock/Cargo.toml",
                "crates/bedrock-core/Cargo.toml",
                LOCKFILE,
            ]
        );
        assert_eq!(changes[0].2, series_file("0.2.0", "0.1.0", "0.2.0-nightly"));
        assert_eq!(changes[2].2, manifest("bedrock", "0.2.0"));
        assert_eq!(changes[3].2, manifest("bedrock-core", "0.2.0"));
        assert_eq!(
            changes[4].2,
            lock.replace("0.1.0-nightly", "0.2.0-nightly")
                .replace("version = \"0.1.0\"", "version = \"0.2.0\"")
        );
        assert_eq!(
            tag_version(&groups, "bedrock", "bedrock-v0.2.0").unwrap(),
            v("0.2.0")
        );
        assert!(tag_version(&groups, "bedrock", "outpost-v0.2.0").is_err());
    }

    /// Scenario _Backport to a series branch_, and the rule advance and
    /// promote share with a patch: a transition on a series moves no library
    /// version, mints no library tag, lands nothing on `main`, and — a
    /// patch's pin moves aside — writes no pin.
    #[test]
    fn transitions_on_a_series_touch_no_library_manifest_pin_or_tag() {
        let groups = workspace_groups();
        for (transition, from) in [
            (Transition::Advance, "0.2.0-pre.1"),
            (Transition::Promote, "0.2.0-pre.7"),
            (Transition::Patch, "0.2.0"),
        ] {
            let plan = plan(transition, from, &[]);
            assert!(
                plan.moves.is_empty() && plan.groups.is_empty(),
                "{transition:?}"
            );
            assert!(plan.pins().is_empty(), "{transition:?}");
            assert_eq!(plan.minted().count(), 0, "{transition:?}");
            assert!(plan.group_tags.is_empty(), "{transition:?}");
            assert_eq!(plan.main_landing(&groups).unwrap(), None, "{transition:?}");
            assert_eq!(
                plan.landing(&groups).unwrap().unwrap().paths,
                "tooling/release-groups.toml,src-tauri/Cargo.toml,crates/cartograph/Cargo.toml,Cargo.lock",
                "{transition:?}"
            );
        }
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
                "the workflow offers `{transition}`, which `release plan` cannot parse; \
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
