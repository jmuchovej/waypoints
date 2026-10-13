//! Plans the four ref-minting transitions of a release cycle.
//!
//! One branch exists per minor version, `v{major}.{minor}.x`. It is cut from
//! `main` as a preview, advances through `-pre.N` while it stabilises, is
//! promoted in place to stable, and then takes patches until the next minor
//! supersedes it. Its role is read from its version, not from a marker file:
//! a prerelease suffix means preview, its absence means stable.
//!
//! Only `cut` consults commit history, because `main` is the only ref whose
//! history describes the work honestly. A release branch carries cherry-picks
//! whose messages describe the original change rather than its severity as a
//! backport, so `advance` and `patch` increment mechanically.
//!
//! Nightlies are not planned here: their version is a timestamp rather than a
//! claim about content. See [`super::nightly_plan`].

use crate::shell;
use anyhow::{Context as _, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use cargo_metadata::semver::{Prerelease, Version};
use clap::{Parser, Subcommand};
use next_version::VersionUpdater;
use strum::{EnumIter, IntoEnumIterator as _};

/// The prerelease identifier every preview carries.
const PRE: &str = "pre";

/// The manifest whose `version` is the app's. Tauri reads it from here when
/// `Tauri.toml` leaves `version` unset.
const APP_MANIFEST: &str = "src-tauri/Cargo.toml";

#[derive(Parser)]
pub struct ReleasePlanArgs {
    #[command(subcommand)]
    transition: Transition,
    /// Plan and print without creating any ref.
    #[arg(long, global = true)]
    dry_run: bool,
}

#[derive(Subcommand, Clone, Copy, Debug, PartialEq, Eq, EnumIter)]
pub enum Transition {
    /// Branch `main` into a new preview line.
    Cut,
    /// Take the next `-pre.N` on the current preview branch.
    Advance,
    /// Drop the prerelease suffix, making this branch stable.
    Promote,
    /// Take the next patch on a stable branch.
    Patch,
}

/// Every transition, in the order a version meets them — the choices the
/// dispatch input offers.
///
/// Derived from the enum rather than restated, so a new variant cannot be
/// missing from the workflow that has to invoke it.
pub fn transitions() -> Vec<String> {
    Transition::iter()
        .map(|transition| format!("{transition:?}").to_lowercase())
        .collect()
}

/// What the planner decided, and what the workflow reads back as job outputs.
///
/// Serialized field-for-field into `$GITHUB_OUTPUT`, so these field names are
/// the output keys.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct Plan {
    /// The version to write into the manifest on `branch`.
    version: String,
    /// The tag to create, always `v{version}`.
    tag: String,
    /// The branch the commit and tag land on.
    branch: String,
    /// `preview` or `stable` — derived from `version`, never stored.
    channel: String,
    /// Where `main` moves to. Empty except on `cut`.
    next_main_version: String,
}

impl Plan {
    fn new(version: Version, next_main: Option<Version>) -> Self {
        Self {
            tag: format!("v{version}"),
            branch: release_branch(&version),
            channel: channel_for(&version).to_owned(),
            next_main_version: next_main.map(|v| v.to_string()).unwrap_or_default(),
            version: version.to_string(),
        }
    }
}

/// The branch a version belongs to. Every patch and preview of one minor
/// shares a branch, which is what lets a preview be promoted in place.
fn release_branch(version: &Version) -> String {
    format!("v{}.{}.x", version.major, version.minor)
}

/// A version names its own channel: a prerelease is a preview, anything else
/// is stable. Deriving it means a build cannot disagree with its version.
fn channel_for(version: &Version) -> &'static str {
    if version.pre.is_empty() {
        "stable"
    } else {
        "preview"
    }
}

/// `0.2.0` → `0.2.0-pre.1`, the first candidate for that release.
fn first_preview(mut target: Version) -> Result<Version> {
    target.pre = Prerelease::new(&format!("{PRE}.1"))
        .with_context(|| format!("`{PRE}.1` is not a valid prerelease"))?;
    Ok(target)
}

/// The target a cut is heading toward, from the work since the last release.
///
/// `main`'s own version is provisional — it is set to a guess right after the
/// previous cut — so the commits decide, not the manifest.
fn cut_target(last_release: &Version, commits: &[String]) -> Version {
    VersionUpdater::new()
        .with_features_always_increment_minor(true)
        .increment(last_release, commits)
}

/// Where `main` goes after a cut: provisionally the next minor, corrected by
/// the next cut's commit history if the cycle turns out smaller.
fn provisional_next_main(target: &Version) -> Version {
    Version::new(target.major, target.minor + 1, 0)
}

fn plan_cut(last_release: &Version, commits: &[String]) -> Result<Plan> {
    let target = cut_target(last_release, commits);
    let next_main = provisional_next_main(&target);
    Ok(Plan::new(first_preview(target)?, Some(next_main)))
}

/// `0.2.0-pre.1` → `0.2.0-pre.2`, `0.2.0-pre` → `0.2.0-pre.1`.
///
/// Counts the trailing identifier directly rather than going through
/// [`VersionUpdater`]: the bumper needs commits to decide anything, and a
/// backport's message describes the original change rather than its severity
/// here. Advancing a preview is mechanical by design.
fn next_preview(current: &Version) -> Result<Version> {
    let pre = current.pre.as_str();
    let (stem, index) = match pre.rsplit_once('.') {
        Some((stem, index)) => match index.parse::<u64>() {
            Ok(index) => (stem, index),
            // A non-numeric tail is part of the name, not a counter.
            Err(_) => (pre, 0),
        },
        None => (pre, 0),
    };

    let mut next = current.clone();
    next.pre = Prerelease::new(&format!("{stem}.{}", index + 1))
        .with_context(|| format!("`{stem}.{}` is not a valid prerelease", index + 1))?;
    Ok(next)
}

fn plan_advance(current: &Version) -> Result<Plan> {
    if current.pre.is_empty() {
        bail!("{current} is already stable; `advance` only applies to a preview");
    }
    Ok(Plan::new(next_preview(current)?, None))
}

/// `0.2.0-pre.7` → `0.2.0`. The branch keeps its identity; only its role moves.
fn plan_promote(current: &Version) -> Result<Plan> {
    if current.pre.is_empty() {
        bail!("{current} is already stable; nothing to promote");
    }
    let mut stable = current.clone();
    stable.pre = Prerelease::EMPTY;
    Ok(Plan::new(stable, None))
}

/// `0.2.0` → `0.2.1`, for a stable branch taking a fix.
fn plan_patch(current: &Version) -> Result<Plan> {
    if !current.pre.is_empty() {
        bail!("{current} is a preview; use `advance` instead of `patch`");
    }
    Ok(Plan::new(
        Version::new(current.major, current.minor, current.patch + 1),
        None,
    ))
}

/// Rewrites the `[package]` version, leaving every other `version =` alone.
///
/// Dependencies carry `version` too, so this walks to the `[package]` table
/// first and edits the first assignment inside it rather than pattern-matching
/// the file.
fn set_package_version(manifest: &str, version: &Version) -> Result<String> {
    let mut in_package = false;
    let mut edited = false;

    let out: Vec<String> = manifest
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('[') {
                in_package = trimmed.starts_with("[package]");
            } else if in_package && !edited && trimmed.starts_with("version") {
                edited = true;
                return format!("version = \"{version}\"");
            }
            line.to_owned()
        })
        .collect();

    if !edited {
        bail!("no `[package]` version to rewrite");
    }
    Ok(out.join("\n") + "\n")
}

// ── Repository state ─────────────────────────────────────────────────────────

/// The newest stable tag, which is what a cut measures its work against.
///
/// Previews and nightlies are excluded: both are views of a release that has
/// not happened yet, so neither marks the end of a cycle.
fn last_stable(tags: &[String]) -> Option<Version> {
    tags.iter()
        .filter_map(|tag| Version::parse(tag.strip_prefix('v')?).ok())
        .filter(|version| version.pre.is_empty())
        .max()
}

fn stable_tags() -> Result<Vec<String>> {
    Ok(shell::git(&["tag", "--list", "v*"])?
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
        Some(version) => format!("v{version}..HEAD"),
        None => "HEAD".to_owned(),
    }
}

/// Conventional-commit subjects for one cycle, oldest first.
///
/// Index commits are left out: they land on `main` between releases, and
/// reading them as work would nudge the target a cut is heading toward.
fn commits_since(since: Option<&Version>) -> Result<Vec<String>> {
    super::release_index::product_commits(&log_range(since))
}

/// One file's blob sha on `branch`, needed to replace it.
fn blob_sha(path: &str, branch: &str) -> Result<String> {
    let repository = env("GITHUB_REPOSITORY")?;
    let response = shell::gh(&[
        "api",
        &format!("repos/{repository}/contents/{path}?ref={branch}"),
        "--jq",
        ".sha",
    ])
    .with_context(|| format!("Failed to read {path} on {branch}"))?;
    Ok(response.trim().to_owned())
}

/// Commits `content` over `path` on `branch`, returning the new commit's sha.
///
/// The Contents API writes a file and commits it in one call, authored by the
/// token's identity — so this needs no `user.name`, no `user.email`, and no
/// push credentials on the checkout.
fn commit_file(path: &str, branch: &str, content: &str, message: &str) -> Result<String> {
    let repository = env("GITHUB_REPOSITORY")?;
    let sha = blob_sha(path, branch)?;

    let response = shell::gh(&[
        "api",
        &format!("repos/{repository}/contents/{path}"),
        "--method",
        "PUT",
        "--raw-field",
        &format!("message={message}"),
        "--raw-field",
        &format!("content={}", BASE64.encode(content)),
        "--raw-field",
        &format!("sha={sha}"),
        "--raw-field",
        &format!("branch={branch}"),
        "--jq",
        ".commit.sha",
    ])
    .with_context(|| format!("Failed to commit {path} on {branch}"))?;

    Ok(response.trim().to_owned())
}

/// Reads the app manifest from `branch`, sets its version, and commits it.
fn write_version(branch: &str, version: &Version, message: &str) -> Result<String> {
    let repository = env("GITHUB_REPOSITORY")?;
    let manifest = shell::gh(&[
        "api",
        &format!("repos/{repository}/contents/{APP_MANIFEST}?ref={branch}"),
        "--jq",
        ".content",
    ])?;

    let decoded = BASE64
        .decode(manifest.replace(['\n', ' '], ""))
        .context("GitHub returned a manifest that is not base64")?;
    let decoded = String::from_utf8(decoded).context("the manifest is not UTF-8")?;

    commit_file(
        APP_MANIFEST,
        branch,
        &set_package_version(&decoded, version)?,
        message,
    )
}

/// Creates a ref through the API, which needs only `GH_TOKEN` — no committer
/// identity and no deep checkout. Mirrors [`super::nightly_plan`].
fn create_ref(kind: &str, name: &str, sha: &str) -> Result<()> {
    let repository = shell::env("GITHUB_REPOSITORY")?;
    shell::gh(&[
        "api",
        &format!("repos/{repository}/git/refs"),
        "--method",
        "POST",
        "--raw-field",
        &format!("ref=refs/{kind}/{name}"),
        "--raw-field",
        &format!("sha={sha}"),
    ])
    .with_context(|| format!("Failed to create refs/{kind}/{name}"))?;
    Ok(())
}

fn env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("${key} is not set"))
}

pub fn run(args: ReleasePlanArgs) -> Result<()> {
    let current = crate::workspace::version()?;

    let plan = match args.transition {
        Transition::Cut => {
            let tags = stable_tags()?;
            let last = last_stable(&tags);
            // Nothing released yet: the first cut is measured from 0.0.0, and
            // `log_range` reads the whole history rather than a missing tag.
            let base = last.clone().unwrap_or_else(|| Version::new(0, 0, 0));
            let commits = commits_since(last.as_ref())?;
            plan_cut(&base, &commits)?
        }
        Transition::Advance => plan_advance(&current)?,
        Transition::Promote => plan_promote(&current)?,
        Transition::Patch => plan_patch(&current)?,
    };

    println!(
        "{transition:?}: {version} on {branch} ({channel}); tag {tag}",
        transition = args.transition,
        version = plan.version,
        branch = plan.branch,
        channel = plan.channel,
        tag = plan.tag,
    );
    if !plan.next_main_version.is_empty() {
        println!("  main moves to {}", plan.next_main_version);
    }

    if args.dry_run {
        println!("  (dry run — nothing written)");
        return crate::utils::github::set_outputs_if_available(&plan);
    }

    // A cut opens the branch before anything can be written to it.
    if args.transition == Transition::Cut {
        create_ref("heads", &plan.branch, &shell::git(&["rev-parse", "HEAD"])?)?;
    }

    // Write the version, then tag *that* commit. Tagging the branch head
    // instead would point the tag at the commit before the bump, leaving the
    // tag and the manifest it names disagreeing.
    let version = Version::parse(&plan.version)?;
    let commit = write_version(
        &plan.branch,
        &version,
        &format!(
            "{branch} {channel} {version}",
            branch = plan.branch,
            channel = plan.channel
        ),
    )?;
    create_ref("tags", &plan.tag, &commit)?;
    println!("  tagged {tag} at {commit}", tag = plan.tag);

    // TODO: if `main` ever becomes a protected branch, botswain will only be
    // able to commit through a PR — adjust accordingly.
    if !plan.next_main_version.is_empty() {
        let next = Version::parse(&plan.next_main_version)?;
        let bumped = write_version("main", &next, &format!("main now targets {next}"))?;
        println!("  main bumped to {next} at {bumped}");
    }

    crate::utils::github::set_outputs_if_available(&plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }
    fn msgs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_version_names_its_own_channel() {
        assert_eq!(channel_for(&v("0.2.0")), "stable");
        assert_eq!(channel_for(&v("0.2.1")), "stable");
        assert_eq!(channel_for(&v("0.2.0-pre.1")), "preview");
    }

    #[test]
    fn one_branch_serves_every_release_of_a_minor() {
        for version in ["0.2.0-pre.1", "0.2.0", "0.2.7"] {
            assert_eq!(release_branch(&v(version)), "v0.2.x");
        }
    }

    /// The index commits land on `main` between releases, and the cut reads
    /// that history — so the target has to come out the same with them there
    /// or not, even though each is worded as a feature.
    #[test]
    fn index_commits_do_not_move_a_cut_target() {
        use super::super::release_index::fixture::Repo;

        let repo = Repo::new();
        repo.product("feat: charts");
        repo.tag("v0.1.0");
        repo.product("fix: a bug");
        let without = repo.product_commits("v0.1.0..HEAD");

        repo.index("stable", "v0.1.0");
        repo.index("nightly", "v0.2.0-nightly.1+2026.08.29");
        let with = repo.product_commits("v0.1.0..HEAD");

        assert_eq!(with, without);
        assert_eq!(cut_target(&v("0.1.0"), &with), v("0.1.1"));
        assert_eq!(
            cut_target(&v("0.1.0"), &with),
            cut_target(&v("0.1.0"), &without)
        );
    }

    #[test]
    fn a_feature_cycle_cuts_the_next_minor() {
        let plan = plan_cut(&v("0.1.0"), &msgs(&["feat: charts", "fix: typo"])).unwrap();
        assert_eq!(plan.version, "0.2.0-pre.1");
        assert_eq!(plan.tag, "v0.2.0-pre.1");
        assert_eq!(plan.branch, "v0.2.x");
        assert_eq!(plan.channel, "preview");
        assert_eq!(plan.next_main_version, "0.3.0");
    }

    #[test]
    fn a_fix_only_cycle_cuts_a_patch() {
        let plan = plan_cut(&v("0.1.0"), &msgs(&["fix: a bug"])).unwrap();
        assert_eq!(plan.version, "0.1.1-pre.1");
        assert_eq!(plan.branch, "v0.1.x");
    }

    #[test]
    fn advancing_a_preview_counts_up() {
        assert_eq!(
            plan_advance(&v("0.2.0-pre.1")).unwrap().version,
            "0.2.0-pre.2"
        );
        // Numeric, not lexical: the tenth candidate must outrank the ninth.
        assert_eq!(
            plan_advance(&v("0.2.0-pre.9")).unwrap().version,
            "0.2.0-pre.10"
        );
        assert!(v("0.2.0-pre.9") < v("0.2.0-pre.10"));
    }

    #[test]
    fn an_uncounted_preview_gains_a_counter() {
        assert_eq!(
            plan_advance(&v("0.2.0-pre")).unwrap().version,
            "0.2.0-pre.1"
        );
    }

    #[test]
    fn promotion_drops_the_suffix_and_keeps_the_branch() {
        let plan = plan_promote(&v("0.2.0-pre.7")).unwrap();
        assert_eq!(plan.version, "0.2.0");
        assert_eq!(plan.tag, "v0.2.0");
        assert_eq!(plan.branch, "v0.2.x");
        assert_eq!(plan.channel, "stable");
    }

    #[test]
    fn patching_stays_on_the_stable_branch() {
        let plan = plan_patch(&v("0.2.0")).unwrap();
        assert_eq!(plan.version, "0.2.1");
        assert_eq!(plan.branch, "v0.2.x");
        assert_eq!(plan.channel, "stable");
    }

    #[test]
    fn transitions_refuse_the_wrong_channel() {
        assert!(plan_advance(&v("0.2.0")).is_err());
        assert!(plan_promote(&v("0.2.0")).is_err());
        assert!(plan_patch(&v("0.2.0-pre.1")).is_err());
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

    /// The manifest the tag names must be the one the tag points at.
    #[test]
    fn the_rewritten_version_is_the_tagged_one() {
        let plan = plan_patch(&v("0.2.0")).unwrap();
        let manifest = "[package]\nversion = \"0.2.0\"\n";
        let out = set_package_version(manifest, &Version::parse(&plan.version).unwrap()).unwrap();
        assert!(out.contains("version = \"0.2.1\""));
        assert_eq!(plan.tag, "v0.2.1");
    }

    /// The dispatch input offers exactly the subcommands clap accepts, so a
    /// run cannot ask for a transition the CLI cannot parse.
    #[test]
    fn every_offered_transition_is_a_real_subcommand() {
        use clap::CommandFactory as _;

        let offered = transitions();
        assert_eq!(offered.len(), Transition::iter().count());

        let subcommands: Vec<String> = ReleasePlanArgs::command()
            .get_subcommands()
            .map(|sub| sub.get_name().to_owned())
            .collect();

        for transition in &offered {
            assert!(
                subcommands.contains(transition),
                "the workflow offers `{transition}`, which `release-plan` cannot parse; \
                 clap knows {subcommands:?}"
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
