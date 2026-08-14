//! Mints the release tag for a Nightly workflow run.
//!
//! Today's date names the tag; the nightly tags already published decide its
//! same-day index. Skips when `HEAD` already carries the latest tag.
//!
//! Tags are read from the checkout with `git` and created through the `gh` API,
//! so the job needs neither deep history nor a committer identity.

use crate::shell;
use anyhow::{Context, Result, bail};
use cargo_metadata::semver::Version;
use chrono::{DateTime, Utc};
use clap::Parser;

#[derive(Parser)]
pub struct NightlyPlanArgs {}

/// What the planner decided, and what the workflow reads back as job outputs.
///
/// Serialized field-for-field into `$GITHUB_OUTPUT`, so these field names are
/// the output keys.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
struct Plan {
    /// Empty on a build-only run.
    tag: String,
    should_build: bool,
    /// Android `versionCode`. Empty when `tag` is.
    version_code: String,
    /// iOS `CFBundleVersion`. Empty when `tag` is.
    bundle_version: String,
}

impl Plan {
    /// Derives the build numbers from the tag, so the two cannot disagree.
    fn build(tag: impl Into<String>) -> Result<Self> {
        let tag = tag.into();
        let (version_code, bundle_version) = if tag.is_empty() {
            (String::new(), String::new())
        } else {
            build_numbers(&tag)?
        };

        Ok(Self {
            tag,
            should_build: true,
            version_code,
            bundle_version,
        })
    }

    const SKIP: Self = Self {
        tag: String::new(),
        should_build: false,
        version_code: String::new(),
        bundle_version: String::new(),
    };
}

pub fn run(_: NightlyPlanArgs) -> Result<()> {
    let head = shell::git(&["rev-parse", "HEAD"])?;
    let plan = plan_next(
        &nightly_tags()?,
        &head,
        &crate::workspace::version()?,
        Utc::now(),
        commit_for,
    )?;

    if !plan.tag.is_empty() {
        create_tag(&plan.tag, &head)?;
        println!("Tagged nightly {tag} at {head}.", tag = plan.tag);
    }

    crate::utils::github::set_outputs(&plan)
}

/// Decides the next tag, or skips when `head` already carries the last one.
///
/// `commit_for` resolves a tag to the commit it points at.
fn plan_next(
    tags: &[String],
    head: &str,
    version: &Version,
    now: DateTime<Utc>,
    commit_for: impl Fn(&str) -> Result<String>,
) -> Result<Plan> {
    if let Some(last) = latest_nightly_tag(tags)
        && commit_for(&last)? == head
    {
        println!("HEAD is already tagged {last}; nothing new to publish.");
        return Ok(Plan::SKIP);
    }

    Plan::build(next_nightly_tag(version, &date_stamp(now), tags)?)
}

/// Nightly tags already published. Requires a checkout with `fetch-tags`.
fn nightly_tags() -> Result<Vec<String>> {
    Ok(shell::git(&["tag", "--list", "v*-nightly.*"])?
        .lines()
        .map(str::to_owned)
        .collect())
}

/// The commit a tag points at, peeled so an annotated tag yields its commit
/// rather than the tag object.
fn commit_for(tag: &str) -> Result<String> {
    shell::git(&["rev-parse", &format!("{tag}^{{commit}}")])
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

/// Today's UTC date as `YYYYMMDD`.
fn date_stamp(now: DateTime<Utc>) -> String {
    now.format("%Y%m%d").to_string()
}

/// How many nightlies one day can carry.
///
/// The Android `versionCode` packs the same-day index into a single decimal
/// digit, so the index must stay within `0..=9`. An eleventh build would wrap:
/// `20260823 * 10 + 10` is `202608240`, which is also the first build of the
/// 24th.
const MAX_NIGHTLIES_PER_DAY: u64 = 10;

/// The store build numbers for a nightly tag, as `(android, ios)`.
///
/// Both encode the UTC date and the tag's own same-day index. Android's
/// `versionCode` is a single integer, so the index becomes its last digit;
/// iOS's `CFBundleVersion` takes period-separated components and needs no
/// packing. Both must increase monotonically or the store rejects the upload.
///
/// ```text
/// v0.2.0-nightly.20260823    ->  202608230   20260823.0
/// v0.2.0-nightly.20260823.1  ->  202608231   20260823.1
/// v0.2.0-nightly.20260823.9  ->  202608239   20260823.9
/// v0.2.0-nightly.20260824    ->  202608240   20260824.0
/// ```
fn build_numbers(tag: &str) -> Result<(String, String)> {
    let suffix = tag
        .split_once("-nightly.")
        .map(|(_, suffix)| suffix)
        .with_context(|| format!("\"{tag}\" is not a nightly tag"))?;

    let (stamp, index) = match suffix.split_once('.') {
        Some((stamp, index)) => (
            stamp,
            index
                .parse::<u64>()
                .with_context(|| format!("\"{tag}\" has a non-numeric same-day index"))?,
        ),
        // An unsuffixed tag is the first build of its day.
        None => (suffix, 0),
    };

    let date: u64 = stamp
        .parse()
        .with_context(|| format!("\"{tag}\" has a non-numeric date stamp"))?;
    if stamp.len() != 8 {
        bail!(
            "\"{tag}\" has a {}-digit date stamp; expected 8",
            stamp.len()
        );
    }
    if index >= MAX_NIGHTLIES_PER_DAY {
        bail!(
            "\"{tag}\" is nightly index {index} of its day; only {MAX_NIGHTLIES_PER_DAY} fit in \
             the version code"
        );
    }

    Ok((format!("{}", date * 10 + index), format!("{stamp}.{index}")))
}

/// The next nightly tag for `stamp`: `v{base}-nightly.{stamp}` for the day's
/// first build, then `.1`, `.2`, and so on — valid, higher-sorting pre-release
/// identifiers that are also the build index.
///
/// `base` is the workspace version with any pre-release suffix stripped. The
/// index is one past the highest already published for that day, so deleting a
/// tag does not free its index.
fn next_nightly_tag(version: &Version, stamp: &str, tags: &[String]) -> Result<String> {
    let base = format!(
        "v{major}.{minor}.{patch}-nightly.{stamp}",
        major = version.major,
        minor = version.minor,
        patch = version.patch,
    );

    let Some(highest) = tags
        .iter()
        .filter_map(|tag| same_day_index(tag, &base))
        .max()
    else {
        return Ok(base);
    };

    // An eleventh tag would yield the same version code as tomorrow's first
    // build. Failing here beats failing at upload.
    let next = highest + 1;
    if next >= MAX_NIGHTLIES_PER_DAY {
        bail!("{MAX_NIGHTLIES_PER_DAY} nightlies already exist for {stamp}");
    }

    Ok(format!("{base}.{next}"))
}

/// The build index `tag` carries for `base`'s day, or `None` if it belongs to
/// another day. The unsuffixed tag is index 0.
fn same_day_index(tag: &str, base: &str) -> Option<u64> {
    match tag.strip_prefix(base)? {
        "" => Some(0),
        rest => rest.strip_prefix('.')?.parse().ok(),
    }
}

/// The most recent `*-nightly.*` tag by semver precedence — the date lives in
/// the tag, so ordering it by version orders it by time.
fn latest_nightly_tag(tags: &[String]) -> Option<String> {
    tags.iter()
        .filter(|tag| tag.contains("-nightly."))
        .filter_map(|tag| {
            Version::parse(tag.strip_prefix('v').unwrap_or(tag))
                .ok()
                .map(|version| (version, tag))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, tag)| tag.clone())
}

fn env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("${name} is not set"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn owned(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|tag| (*tag).to_owned()).collect()
    }

    /// The workspace version as `cargo metadata` hands it over.
    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn date_stamp_formats_a_utc_date() {
        let now = Utc.with_ymd_and_hms(2026, 7, 26, 23, 59, 59).unwrap();
        assert_eq!(date_stamp(now), "20260726");
    }

    /// The ordinal is 0-based in both encodings, so an unsuffixed tag — the
    /// scheduled build — ends in `0` rather than `1`.
    #[test]
    fn build_numbers_are_zero_based() {
        let (android, ios) = build_numbers("v0.2.0-nightly.20260823").unwrap();
        assert_eq!(
            (android.as_str(), ios.as_str()),
            ("202608230", "20260823.0")
        );
    }

    /// The tag's suffix is the index, so it passes through unchanged.
    #[test]
    fn build_numbers_track_the_same_day_index() {
        for (tag, android, ios) in [
            ("v0.2.0-nightly.20260823.1", "202608231", "20260823.1"),
            ("v0.2.0-nightly.20260823.9", "202608239", "20260823.9"),
        ] {
            let (got_android, got_ios) = build_numbers(tag).unwrap();
            assert_eq!(
                (got_android.as_str(), got_ios.as_str()),
                (android, ios),
                "{tag}"
            );
        }
    }

    /// The property the whole scheme exists for: a full day of forced builds
    /// still leaves the next day's scheduled build strictly higher.
    #[test]
    fn the_day_rolls_over_without_colliding() {
        let (last_of_day, last_ios) = build_numbers("v0.2.0-nightly.20260823.9").unwrap();
        let (next_day, next_ios) = build_numbers("v0.2.0-nightly.20260824").unwrap();

        assert!(
            next_day.parse::<u64>().unwrap() > last_of_day.parse::<u64>().unwrap(),
            "{next_day} must exceed {last_of_day}"
        );
        assert_eq!(
            (last_ios.as_str(), next_ios.as_str()),
            ("20260823.9", "20260824.0")
        );
    }

    /// Index 10 would pack two digits into one and land on the next day's first
    /// version code, so it is refused rather than silently wrapped.
    #[test]
    fn an_eleventh_same_day_build_is_refused() {
        let error = build_numbers("v0.2.0-nightly.20260823.10").unwrap_err();
        assert!(error.to_string().contains("only 10"), "{error}");
    }

    #[test]
    fn next_tag_refuses_to_exceed_the_daily_cap() {
        let existing: Vec<String> = std::iter::once("v0.2.0-nightly.20260823".to_string())
            .chain(
                (1..MAX_NIGHTLIES_PER_DAY).map(|index| format!("v0.2.0-nightly.20260823.{index}")),
            )
            .collect();

        let error = next_nightly_tag(&version("0.2.0"), "20260823", &existing).unwrap_err();
        assert!(error.to_string().contains("already exist"), "{error}");
    }

    /// Every tag the planner can mint must yield a usable pair, so the two
    /// functions cannot drift apart.
    #[test]
    fn every_tag_the_planner_mints_has_build_numbers() {
        let mut existing: Vec<String> = Vec::new();
        for _ in 0..MAX_NIGHTLIES_PER_DAY {
            let tag = next_nightly_tag(&version("0.2.0"), "20260823", &existing).unwrap();
            build_numbers(&tag).unwrap_or_else(|e| panic!("{tag}: {e}"));
            existing.push(tag);
        }
    }

    #[test]
    fn non_nightly_tags_are_rejected() {
        let error = build_numbers("v0.2.0").unwrap_err();
        assert!(error.to_string().contains("not a nightly tag"), "{error}");
    }

    #[test]
    fn next_tag_stamps_the_base_version() {
        let tag = next_nightly_tag(&version("0.2.0"), "20260726", &owned(&[])).unwrap();
        assert_eq!(tag, "v0.2.0-nightly.20260726");
    }

    #[test]
    fn next_tag_strips_a_pre_release_suffix() {
        let tag = next_nightly_tag(&version("0.2.0-pre.3"), "20260726", &owned(&[])).unwrap();
        assert_eq!(tag, "v0.2.0-nightly.20260726");
    }

    #[test]
    fn next_tag_disambiguates_a_same_day_collision() {
        let tag = next_nightly_tag(
            &version("0.2.0"),
            "20260726",
            &owned(&["v0.2.0-nightly.20260726"]),
        )
        .unwrap();
        assert_eq!(tag, "v0.2.0-nightly.20260726.1");
    }

    #[test]
    fn next_tag_walks_past_multiple_collisions() {
        let existing = owned(&["v0.2.0-nightly.20260726", "v0.2.0-nightly.20260726.1"]);
        let tag = next_nightly_tag(&version("0.2.0"), "20260726", &existing).unwrap();
        assert_eq!(tag, "v0.2.0-nightly.20260726.2");
    }

    /// The disambiguated form has to keep sorting above the tag it disambiguates,
    /// or `latest_nightly_tag` would walk backwards.
    #[test]
    fn disambiguated_tags_sort_above_their_base() {
        let tags = owned(&["v0.2.0-nightly.20260726", "v0.2.0-nightly.20260726.1"]);
        assert_eq!(
            latest_nightly_tag(&tags).as_deref(),
            Some("v0.2.0-nightly.20260726.1")
        );
    }

    #[test]
    fn latest_ignores_non_nightly_tags() {
        let tags = owned(&[
            "v0.2.0-nightly.20260725",
            "v0.1.0",
            "v0.2.0-nightly.20260726",
            "v0.2.0-nightly.20260726.1",
            "v0.2.0-pre.1",
        ]);
        assert_eq!(
            latest_nightly_tag(&tags).as_deref(),
            Some("v0.2.0-nightly.20260726.1")
        );
    }

    /// Ordering is by version, not lexical — `0.10.0` outranks `0.9.0` even
    /// though its date stamp is older.
    #[test]
    fn latest_orders_across_base_versions() {
        let tags = owned(&["v0.9.0-nightly.20260101", "v0.10.0-nightly.20251231"]);
        assert_eq!(
            latest_nightly_tag(&tags).as_deref(),
            Some("v0.10.0-nightly.20251231")
        );
    }

    #[test]
    fn latest_is_none_without_nightly_tags() {
        assert_eq!(
            latest_nightly_tag(&owned(&["v0.1.0", "v0.2.0-pre.1"])),
            None
        );
    }

    /// Stands in for `commit_for`: every tag resolves to `sha`.
    fn tagging(sha: &str) -> impl Fn(&str) -> Result<String> + '_ {
        move |_| Ok(sha.to_owned())
    }

    fn at(stamp: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!(
            "{}-{}-{}T04:00:00Z",
            &stamp[..4],
            &stamp[4..6],
            &stamp[6..]
        ))
        .unwrap()
        .with_timezone(&Utc)
    }

    #[test]
    fn skips_when_head_is_already_tagged() {
        let tags = owned(&["v0.2.0-nightly.20260725"]);
        let plan = plan_next(
            &tags,
            "SHA",
            &version("0.2.0"),
            at("20260726"),
            tagging("SHA"),
        )
        .unwrap();
        assert_eq!(plan, Plan::SKIP);
    }

    #[test]
    fn tags_when_head_has_moved() {
        let tags = owned(&["v0.2.0-nightly.20260725"]);
        let plan = plan_next(
            &tags,
            "NEW",
            &version("0.2.0"),
            at("20260726"),
            tagging("OLD"),
        )
        .unwrap();
        assert_eq!(plan, Plan::build("v0.2.0-nightly.20260726").unwrap());
    }

    /// The very first nightly has nothing to compare against, so it must build
    /// rather than skip.
    #[test]
    fn tags_when_no_nightly_exists_yet() {
        let tags = owned(&["v0.1.0"]);
        let plan = plan_next(
            &tags,
            "SHA",
            &version("0.2.0"),
            at("20260726"),
            tagging("SHA"),
        )
        .unwrap();
        assert_eq!(plan, Plan::build("v0.2.0-nightly.20260726").unwrap());
    }

    /// A dispatch after the cron has already run, or vice versa — the day’s next
    /// index, which is what makes the suffix reachable at all.
    #[test]
    fn a_second_nightly_the_same_day_takes_the_next_index() {
        let tags = owned(&["v0.2.0-nightly.20260726"]);
        let plan = plan_next(
            &tags,
            "NEW",
            &version("0.2.0"),
            at("20260726"),
            tagging("OLD"),
        )
        .unwrap();
        assert_eq!(plan, Plan::build("v0.2.0-nightly.20260726.1").unwrap());
    }

    /// The counterpart: a dispatch on sources already published republishes
    /// nothing, even though the cap for the day is nowhere near reached.
    #[test]
    fn a_second_nightly_the_same_day_still_needs_commits() {
        let tags = owned(&["v0.2.0-nightly.20260726"]);
        let plan = plan_next(
            &tags,
            "SHA",
            &version("0.2.0"),
            at("20260726"),
            tagging("SHA"),
        )
        .unwrap();
        assert_eq!(plan, Plan::SKIP);
    }

    /// The read paths, against this checkout — no network, no fixtures. Any
    /// tag the filter admits must be one `build_numbers` can parse, or a
    /// nightly run would fail after the tag already exists.
    #[test]
    fn reads_tags_and_resolves_commits_from_the_checkout() {
        const SHA_LENGTH: usize = 40;

        let head = shell::git(&["rev-parse", "HEAD"]).unwrap();
        assert_eq!(head.len(), SHA_LENGTH, "expected a full sha, got {head:?}");

        for tag in nightly_tags().unwrap() {
            build_numbers(&tag).unwrap_or_else(|error| panic!("{tag}: {error}"));
            assert_eq!(commit_for(&tag).unwrap().len(), SHA_LENGTH, "{tag}");
        }
    }

    /// `Plan` is serialized field-for-field into `$GITHUB_OUTPUT`, and the
    /// `prepare` job republishes those keys as job outputs. Renaming a field
    /// here without renaming its `StepOutput` there leaves the job output
    /// resolving to an empty string.
    #[test]
    fn the_prepare_job_republishes_exactly_the_plans_fields() {
        use crate::tasks::workflows::jobs;
        use gh_workflow::NamedJob;
        use std::collections::BTreeSet;

        let serialized = serde_json::to_value(Plan::SKIP).unwrap();
        let fields: BTreeSet<&str> = serialized
            .as_object()
            .expect("a Plan serializes to an object")
            .keys()
            .map(String::as_str)
            .collect();

        let prepare: NamedJob = jobs::prepare().into();
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
