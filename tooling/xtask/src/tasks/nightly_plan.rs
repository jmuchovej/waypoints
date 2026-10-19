//! Mints the release tag for a Nightly workflow run.
//!
//! A nightly is `{target}-nightly.{N}+{YYYY.MM.DD}`. `N` counts the nightlies
//! shipped since the target last changed, and carries the semver ordering;
//! the date is build metadata, which semver ignores for precedence, so it is
//! free to be documentation that the store build numbers are also read from.
//!
//! Splitting the two is what keeps this simple. `N` is bumped by
//! [`VersionUpdater`], which increments a trailing numeric prerelease
//! identifier and leaves build metadata alone — and, usefully, returns the
//! version *unchanged* when no commits landed, so "nothing new to publish" and
//! "what is the next version" are one question. The date is then stamped over
//! whatever metadata came through, because a preserved date is a stale one.
//!
//! The date being the store build number is also the rate limit: two runs on
//! one day would produce the same `versionCode`, so a nightly ships at most
//! once a day. A manual dispatch therefore builds without publishing — but it
//! still resolves the version and build numbers the scheduled run would use,
//! so its artifacts are stamped like real nightlies and can be uploaded by
//! hand for testing.
//!
//! Tags are read from the checkout with `git` and created through the `gh` API,
//! so the job needs neither deep history nor a committer identity.

use crate::shell;
use anyhow::{Context, Result, bail};
use cargo_metadata::semver::{BuildMetadata, Prerelease, Version};
use chrono::{DateTime, Utc};
use clap::Parser;
use next_version::VersionUpdater;

/// The prerelease identifier every nightly carries.
const NIGHTLY: &str = "nightly";

#[derive(Parser)]
pub struct NightlyPlanArgs {
    /// Build without publishing: no tag is minted and nothing is uploaded.
    ///
    /// The date doubles as the store build number, so a second publish on one
    /// day could not produce a distinct one. A dispatch exercises the pipeline
    /// instead, and the next scheduled run publishes. The artifacts still carry
    /// the version and build numbers that run would mint, so they can be
    /// uploaded by hand.
    #[arg(long)]
    build_only: bool,
}

/// What the planner decided, and what the workflow reads back as job outputs.
///
/// Serialized field-for-field into `$GITHUB_OUTPUT`, so these field names are
/// the output keys.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
struct Plan {
    /// Empty on a build-only run.
    tag: String,
    /// Whether this run itself created `tag`'s ref — true only on the
    /// publish path, which always calls [`create_tag`] right after. A
    /// withdraw job reads this back to decide whether it may delete the
    /// tag: this run's own ref, or one that predates it (a tag push, for
    /// the channels that have no planner at all).
    tag_minted: bool,
    should_build: bool,
    /// The full nightly version, `{target}-nightly.{N}+{date}`, which the
    /// builds stamp into the app. Empty only when nothing builds.
    version: String,
    /// Android `versionCode`. Empty when `version` is.
    version_code: String,
    /// iOS `CFBundleVersion`. Empty when `version` is.
    bundle_version: String,
}

impl Plan {
    /// Derives the build numbers from the version, so the two cannot disagree.
    fn publish(version: &Version) -> Result<Self> {
        Ok(Self {
            tag: format!("v{version}"),
            tag_minted: true,
            ..Self::build_only(version)?
        })
    }

    /// Builds `version` without minting its tag — a manual dispatch. The
    /// artifacts are stamped exactly as the scheduled run would stamp them.
    fn build_only(version: &Version) -> Result<Self> {
        let (version_code, bundle_version) = build_numbers(version)?;
        Ok(Self {
            tag: String::new(),
            tag_minted: false,
            should_build: true,
            version: version.to_string(),
            version_code,
            bundle_version,
        })
    }

    const SKIP: Self = Self {
        tag: String::new(),
        tag_minted: false,
        should_build: false,
        version: String::new(),
        version_code: String::new(),
        bundle_version: String::new(),
    };
}

/// Today's UTC date, as the build metadata a nightly carries.
fn date_stamp(now: DateTime<Utc>) -> String {
    now.format("%Y.%m.%d").to_string()
}

/// The store build numbers for a nightly, as `(android, ios)`.
///
/// Both are the date: Android's `versionCode` is a single integer, so the
/// separators are dropped; iOS's `CFBundleVersion` takes period-separated
/// components and keeps them. Both increase monotonically because dates do,
/// and 8 digits stays far under Android's 2_100_000_000 ceiling.
///
/// ```text
/// 0.2.0-nightly.1+2026.08.29  ->  20260829   2026.08.29
/// 0.2.0-nightly.2+2026.08.30  ->  20260830   2026.08.30
/// ```
fn build_numbers(version: &Version) -> Result<(String, String)> {
    let stamp = version.build.as_str();
    if stamp.is_empty() {
        bail!("{version} carries no date; a nightly needs one for its build number");
    }

    let android: String = stamp.chars().filter(char::is_ascii_digit).collect();
    if android.len() != 8 {
        bail!(
            "\"{stamp}\" is not a YYYY.MM.DD date; got {} digits",
            android.len()
        );
    }

    Ok((android, stamp.to_owned()))
}

/// The target a nightly previews: the version with any suffix stripped.
///
/// `N` counts within one target, so a bump on `main` restarts it. That is safe
/// because the store build number is the date, not `N`.
fn target(version: &Version) -> Version {
    Version::new(version.major, version.minor, version.patch)
}

/// The newest nightly already published for `target`.
fn latest_nightly(target: &Version, tags: &[String]) -> Option<Version> {
    tags.iter()
        .filter_map(|tag| Version::parse(tag.strip_prefix('v')?).ok())
        .filter(|version| version.pre.starts_with(NIGHTLY) && self::target(version) == *target)
        .max()
}

/// The first nightly of a target, `{target}-nightly.1`.
fn first_nightly(target: &Version) -> Result<Version> {
    let mut version = target.clone();
    version.pre = Prerelease::new(&format!("{NIGHTLY}.1"))
        .with_context(|| format!("`{NIGHTLY}.1` is not a valid prerelease"))?;
    Ok(version)
}

/// The next nightly, or `None` when nothing landed since the last one.
///
/// [`VersionUpdater`] returns the version unchanged given no commits, which is
/// exactly the skip condition — so the bump and the "is there anything new?"
/// check are the same call.
fn next_nightly(
    latest: Option<&Version>,
    commits: &[String],
    target: &Version,
    stamp: &str,
) -> Result<Option<Version>> {
    let next = match latest {
        None => first_nightly(target)?,
        Some(latest) => {
            let bumped = VersionUpdater::new().increment(latest, commits);
            if bumped.pre == latest.pre {
                return Ok(None);
            }
            bumped
        }
    };

    Ok(Some(stamped(next, stamp)?))
}

/// `version` carrying `stamp` as its build metadata. Whatever metadata came
/// through is the previous run's date.
fn stamped(mut version: Version, stamp: &str) -> Result<Version> {
    version.build = BuildMetadata::new(stamp)
        .with_context(|| format!("\"{stamp}\" is not valid build metadata"))?;
    Ok(version)
}

/// What a build-only run stamps its artifacts with: the next nightly when
/// something landed, else the latest one restamped with today's date — a
/// rebuild of the nightly that is current, not a new one.
fn build_only_version(
    latest: Option<&Version>,
    commits: &[String],
    target: &Version,
    stamp: &str,
) -> Result<Version> {
    match next_nightly(latest, commits, target, stamp)? {
        Some(next) => Ok(next),
        None => stamped(
            latest
                .cloned()
                .expect("next_nightly only declines a bump when a latest nightly exists"),
            stamp,
        ),
    }
}

fn nightly_tags() -> Result<Vec<String>> {
    Ok(shell::git(&["tag", "--list", "v*-nightly.*"])?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Commit subjects since `tag`, or the whole history before the first nightly.
///
/// Index commits are left out: the entry recording a nightly lands right after
/// its tag, and counting it would justify the next nightly every day.
fn commits_since(tag: Option<&Version>) -> Result<Vec<String>> {
    let range = match tag {
        Some(version) => format!("v{version}..HEAD"),
        None => "HEAD".to_owned(),
    };
    super::release_index::product_commits(&range)
}

/// Creates the tag ref through the API, which needs only `GH_TOKEN`.
fn create_tag(tag: &str, sha: &str) -> Result<()> {
    let repository = env("GITHUB_REPOSITORY")?;
    shell::gh(&[
        "api",
        &format!("repos/{repository}/git/refs"),
        "--method",
        "POST",
        "--raw-field",
        &format!("ref=refs/tags/{tag}"),
        "--raw-field",
        &format!("sha={sha}"),
    ])
    .with_context(|| format!("Failed to create refs/tags/{tag}"))?;

    Ok(())
}

fn env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("${key} is not set"))
}

pub fn run(args: NightlyPlanArgs) -> Result<()> {
    let target = target(&crate::workspace::version()?);
    let tags = nightly_tags()?;
    let latest = latest_nightly(&target, &tags);
    let commits = commits_since(latest.as_ref())?;
    let stamp = date_stamp(Utc::now());

    if args.build_only {
        let version = build_only_version(latest.as_ref(), &commits, &target, &stamp)?;
        println!("Manual dispatch: building {version} without publishing.");
        return crate::utils::github::set_outputs(&Plan::build_only(&version)?);
    }

    let plan = match next_nightly(latest.as_ref(), &commits, &target, &stamp)? {
        None => {
            println!("Nothing landed since the last nightly; nothing new to publish.");
            Plan::SKIP
        }
        Some(version) => {
            let head = shell::git(&["rev-parse", "HEAD"])?;
            let plan = Plan::publish(&version)?;
            create_tag(&plan.tag, &head)?;
            println!("Tagged nightly {tag} at {head}.", tag = plan.tag);
            plan
        }
    };

    crate::utils::github::set_outputs(&plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }
    fn owned(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|tag| (*tag).to_owned()).collect()
    }
    fn commit() -> Vec<String> {
        vec!["fix: something landed".to_owned()]
    }

    #[test]
    fn date_stamp_formats_a_utc_date() {
        let now = Utc.with_ymd_and_hms(2026, 7, 26, 23, 59, 59).unwrap();
        assert_eq!(date_stamp(now), "2026.07.26");
    }

    #[test]
    fn build_numbers_come_from_the_date() {
        let (android, ios) = build_numbers(&v("0.2.0-nightly.7+2026.08.29")).unwrap();
        assert_eq!((android.as_str(), ios.as_str()), ("20260829", "2026.08.29"));
    }

    /// Android's `versionCode` is one integer and must only ever climb.
    #[test]
    fn version_codes_increase_with_the_date() {
        let earlier = build_numbers(&v("0.2.0-nightly.9+2026.08.29")).unwrap().0;
        let later = build_numbers(&v("0.2.0-nightly.1+2026.09.01")).unwrap().0;
        assert!(earlier.parse::<u64>().unwrap() < later.parse::<u64>().unwrap());
        // ...even across a target bump, which restarts N but not the date.
        assert!(later.parse::<u64>().unwrap() < 2_100_000_000);
    }

    #[test]
    fn a_nightly_without_a_date_is_rejected() {
        assert!(build_numbers(&v("0.2.0-nightly.1")).is_err());
    }

    /// A dispatch mints no tag but stamps its artifacts like the real run.
    #[test]
    fn a_build_only_plan_carries_the_numbers_without_a_tag() {
        let plan = Plan::build_only(&v("0.2.0-nightly.7+2026.08.29")).unwrap();
        assert_eq!(plan.tag, "");
        assert!(plan.should_build);
        assert_eq!(plan.version, "0.2.0-nightly.7+2026.08.29");
        assert_eq!(plan.version_code, "20260829");
        assert_eq!(plan.bundle_version, "2026.08.29");

        let published = Plan::publish(&v("0.2.0-nightly.7+2026.08.29")).unwrap();
        assert_eq!(published.tag, "v0.2.0-nightly.7+2026.08.29");
        assert_eq!(published.version_code, plan.version_code);
    }

    /// `tag_minted` is true only on the path that actually creates the ref
    /// (`Plan::publish`, immediately followed by `create_tag` in `run`) — a
    /// withdraw job reads this back to know whether it may delete the tag.
    #[test]
    fn tag_minted_is_set_only_on_the_publish_path() {
        let version = v("0.2.0-nightly.7+2026.08.29");
        assert!(Plan::publish(&version).unwrap().tag_minted);
        assert!(!Plan::build_only(&version).unwrap().tag_minted);
        assert!(!Plan::SKIP.tag_minted);
    }

    /// Nothing landed: a dispatch rebuilds the current nightly under today's
    /// date rather than inventing a counter the scheduled run would not.
    #[test]
    fn an_idle_dispatch_rebuilds_the_latest_nightly_under_todays_date() {
        let latest = v("0.2.0-nightly.3+2026.08.29");
        let version = build_only_version(Some(&latest), &[], &v("0.2.0"), "2026.08.30").unwrap();
        assert_eq!(version.to_string(), "0.2.0-nightly.3+2026.08.30");

        let version =
            build_only_version(Some(&latest), &commit(), &v("0.2.0"), "2026.08.30").unwrap();
        assert_eq!(version.to_string(), "0.2.0-nightly.4+2026.08.30");

        let version = build_only_version(None, &[], &v("0.3.0"), "2026.08.30").unwrap();
        assert_eq!(version.to_string(), "0.3.0-nightly.1+2026.08.30");
    }

    #[test]
    fn the_first_nightly_of_a_target_starts_at_one() {
        let next = next_nightly(None, &[], &v("0.2.0"), "2026.08.29")
            .unwrap()
            .unwrap();
        assert_eq!(next.to_string(), "0.2.0-nightly.1+2026.08.29");
    }

    #[test]
    fn a_landed_commit_advances_the_counter() {
        let latest = v("0.2.0-nightly.1+2026.08.29");
        let next = next_nightly(Some(&latest), &commit(), &v("0.2.0"), "2026.08.30")
            .unwrap()
            .unwrap();
        assert_eq!(next.to_string(), "0.2.0-nightly.2+2026.08.30");
    }

    /// The bump and the "anything new?" check are the same call.
    #[test]
    fn an_idle_day_publishes_nothing() {
        let latest = v("0.2.0-nightly.1+2026.08.29");
        let next = next_nightly(Some(&latest), &[], &v("0.2.0"), "2026.08.30").unwrap();
        assert_eq!(next, None);
    }

    /// The entry recording a nightly lands right after its tag. Were it to
    /// count as work, every quiet day would publish a nightly.
    #[test]
    fn an_index_commit_does_not_wake_a_quiet_repository() {
        use super::super::release_index::fixture::Repo;

        let latest = v("0.2.0-nightly.1+2026.08.29");
        let range = "v0.2.0-nightly.1+2026.08.29..HEAD";

        let repo = Repo::new();
        repo.product("feat: the nightly's work");
        repo.tag("v0.2.0-nightly.1+2026.08.29");
        repo.index("nightly", "v0.2.0-nightly.1+2026.08.29");

        let commits = repo.product_commits(range);
        assert!(commits.is_empty(), "{commits:?}");
        let next = next_nightly(Some(&latest), &commits, &v("0.2.0"), "2026.08.30").unwrap();
        assert_eq!(next, None);

        // ...while real work alongside it still publishes.
        repo.product("fix: something landed");
        let commits = repo.product_commits(range);
        assert_eq!(commits, ["fix: something landed"]);
        let next = next_nightly(Some(&latest), &commits, &v("0.2.0"), "2026.08.30")
            .unwrap()
            .unwrap();
        assert_eq!(next.to_string(), "0.2.0-nightly.2+2026.08.30");
    }

    /// A preserved date would be the previous run's.
    #[test]
    fn the_date_is_restamped_not_carried() {
        let latest = v("0.2.0-nightly.1+2026.08.29");
        let next = next_nightly(Some(&latest), &commit(), &v("0.2.0"), "2026.09.01")
            .unwrap()
            .unwrap();
        assert_eq!(next.build.as_str(), "2026.09.01");
    }

    #[test]
    fn the_counter_restarts_when_the_target_moves() {
        let tags = owned(&["v0.2.0-nightly.9+2026.08.29"]);
        assert_eq!(latest_nightly(&v("0.3.0"), &tags), None);
        let next = next_nightly(None, &commit(), &v("0.3.0"), "2026.08.30")
            .unwrap()
            .unwrap();
        assert_eq!(next.to_string(), "0.3.0-nightly.1+2026.08.30");
    }

    #[test]
    fn the_latest_nightly_is_the_highest_counter_not_the_newest_date() {
        let tags = owned(&[
            "v0.2.0-nightly.1+2026.08.29",
            "v0.2.0-nightly.10+2026.08.30",
            "v0.2.0-nightly.9+2026.09.01",
        ]);
        let latest = latest_nightly(&v("0.2.0"), &tags).unwrap();
        assert_eq!(latest.pre.as_str(), "nightly.10");
    }

    #[test]
    fn non_nightly_tags_are_ignored() {
        let tags = owned(&["v0.2.0", "v0.2.0-pre.3", "v0.2.0-nightly.4+2026.08.29"]);
        assert_eq!(
            latest_nightly(&v("0.2.0"), &tags).unwrap().pre.as_str(),
            "nightly.4"
        );
    }

    /// Build metadata is ignored for precedence, so `N` must carry the order.
    #[test]
    fn the_counter_orders_nightlies_not_the_date() {
        assert!(v("0.2.0-nightly.9+2026.09.01") < v("0.2.0-nightly.10+2026.08.29"));
    }

    /// nightly < preview < stable, and the next target outranks the release.
    #[test]
    fn channels_stay_ordered() {
        let ordered = [
            v("0.2.0-nightly.1+2026.08.29"),
            v("0.2.0-nightly.2+2026.08.30"),
            v("0.2.0-pre.1"),
            v("0.2.0"),
            v("0.3.0-nightly.1+2026.09.01"),
        ];
        let mut sorted = ordered.to_vec();
        sorted.sort();
        assert_eq!(sorted, ordered, "channel ordering is not monotonic");
    }

    /// A dispatch builds but publishes nothing; an idle day does neither.
    #[test]
    fn the_three_outcomes_differ_in_the_right_way() {
        let outcome = |plan: &Plan| (plan.should_build, plan.tag.is_empty());
        let version = v("0.2.0-nightly.1+2026.08.29");
        assert_eq!(outcome(&Plan::build_only(&version).unwrap()), (true, true));
        assert_eq!(outcome(&Plan::SKIP), (false, true));
        assert_eq!(outcome(&Plan::publish(&version).unwrap()), (true, false));
    }

    /// `Plan` is serialized field-for-field into `$GITHUB_OUTPUT`, and the
    /// `prepare` job republishes those keys as job outputs. Renaming a field
    /// here without renaming its `StepOutput` there leaves the job output
    /// resolving to an empty string.
    #[test]
    fn the_prepare_job_republishes_exactly_the_plans_fields() {
        use crate::tasks::workflows::{WorkflowContext, jobs};
        use gh_workflow::NamedJob;
        use std::collections::BTreeSet;

        let serialized = serde_json::to_value(Plan::SKIP).unwrap();
        let fields: BTreeSet<&str> = serialized
            .as_object()
            .expect("a Plan serializes to an object")
            .keys()
            .map(String::as_str)
            .collect();

        let prepare: NamedJob = jobs::prepare(&WorkflowContext::fixture()).into();
        let republished: BTreeSet<&str> = prepare
            .job
            .outputs
            .as_ref()
            .expect("the prepare job declares outputs")
            .keys()
            .map(String::as_str)
            .collect();

        assert_eq!(fields, republished);
    }
}
