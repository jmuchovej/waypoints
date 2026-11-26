// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decides what a nightly bump mints.
//!
//! A nightly is `{target}-nightly.{YYYYMMDD}`: the planner's UTC date is the
//! whole prerelease identifier, so nightlies order by date under semver alone
//! and a day holds at most one. The same date is the store build number,
//! which is the same limit stated the other way round — two runs on one day
//! would produce the same `versionCode`. A dispatched nightly bump therefore
//! rehearses: it decides as the scheduled run would and mints nothing.
//!
//! Tags are read from the remote and created through the API, so the job
//! needs neither a committer identity nor a checkout whose tags are current.

use crate::{
    releases::{index, version::Version},
    tools::git,
};
use anyhow::{Context as _, Result, bail};
use chrono::NaiveDate;

/// The nightly a bump mints, or would mint: what the workflow reads back as
/// outputs.
///
/// Serialized field-for-field into `$GITHUB_OUTPUT`, so these field names are
/// the output keys.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct Plan {
    pub tag: String,
    pub version: String,
    /// Android `versionCode`: the nightly's date.
    pub version_code: String,
    /// iOS `CFBundleVersion`. As `version_code`.
    pub bundle_version: String,
}

impl Plan {
    /// Derives the build numbers from the version, so the two cannot disagree.
    pub fn of(version: &Version) -> Result<Self> {
        let (version_code, bundle_version) = version.build_numbers()?;
        Ok(Self {
            tag: version.tag(),
            version: version.to_string(),
            version_code,
            bundle_version,
        })
    }
}

/// What a run publishes, given what is already published and what landed.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Work landed, or nothing is published yet: today's nightly. A
    /// scheduled run publishes it.
    New(Version),
    /// Nothing landed: the published nightly, as it is. A scheduled run
    /// publishes nothing.
    Current(Version),
    /// Work landed, but today's nightly already exists: the stores hold the
    /// day's build, and the work ships with the next scheduled nightly. A
    /// scheduled run publishes nothing.
    Taken(Version),
}

impl Decision {
    /// The version the decision is about.
    pub fn version(&self) -> &Version {
        match self {
            Decision::New(version) | Decision::Current(version) | Decision::Taken(version) => {
                version
            }
        }
    }
}

/// Decides from `latest` (the newest nightly published for `target`) and
/// `commits` (the work since it) what a run dated `today` publishes.
pub fn decide(
    latest: Option<&Version>,
    commits: &[String],
    target: &Version,
    today: NaiveDate,
) -> Result<Decision> {
    let todays = target.nightly(today)?;
    let Some(latest) = latest else {
        return Ok(Decision::New(todays));
    };
    if todays < *latest {
        bail!("the latest nightly {latest} is dated after today's {todays}");
    }
    Ok(if commits.is_empty() {
        Decision::Current(latest.clone())
    } else if todays == *latest {
        Decision::Taken(latest.clone())
    } else {
        Decision::New(todays)
    })
}

/// The newest nightly already published for `target`.
///
/// A tag that names a nightly without fitting the grammar fails the planner
/// by name: a stale shape must not vote silently. Tags of other channels, and
/// tags that are not versions at all, are not nightlies and are passed over.
pub fn latest_nightly(target: &Version, tags: &[String]) -> Result<Option<Version>> {
    let mut latest: Option<Version> = None;
    for tag in tags {
        let Ok(version) = tag.parse::<Version>() else {
            continue;
        };
        if !version.is_nightly() {
            continue;
        }
        version
            .nightly_date()
            .with_context(|| format!("tag `{tag}` is not a dated nightly"))?;
        if version.target() == *target && latest.as_ref().is_none_or(|current| *current < version) {
            latest = Some(version);
        }
    }
    Ok(latest)
}

/// The published tags, from the remote: a workstation's local tags can be
/// stale or stray, and a decision is a claim about what is published.
pub fn nightly_tags() -> Result<Vec<String>> {
    git::remote_tags()
}

/// Commit subjects since `tag`, or the whole history before the first nightly.
///
/// Index commits are left out: the entry recording a nightly lands right after
/// its tag, and counting it would justify the next nightly every day.
pub fn commits_since(tag: Option<&Version>) -> Result<Vec<String>> {
    let range = match tag {
        Some(version) => format!("{}..HEAD", version.tag()),
        None => "HEAD".to_owned(),
    };
    index::product_commits(&range)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }
    fn owned(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|tag| (*tag).to_owned()).collect()
    }
    fn commit() -> Vec<String> {
        vec!["fix: something landed".to_owned()]
    }
    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    /// Scenario _A dry run reports the would-be tag_: the plan carries the
    /// tag and the version with the store build numbers the date is, and
    /// nothing of whether it was minted — the report says that.
    #[test]
    fn a_plan_carries_the_tag_and_its_build_numbers() {
        let plan = Plan::of(&v("0.2.0-nightly.20260829")).unwrap();
        assert_eq!(plan.tag, "v0.2.0-nightly.20260829");
        assert_eq!(plan.version, "0.2.0-nightly.20260829");
        assert_eq!(plan.version_code, "20260829");
        assert_eq!(plan.bundle_version, "2026.08.29");
    }

    #[test]
    fn the_first_nightly_of_a_target_is_new() {
        let decision = decide(None, &[], &v("0.2.0"), day(2026, 8, 29)).unwrap();
        assert_eq!(decision, Decision::New(v("0.2.0-nightly.20260829")));
    }

    #[test]
    fn a_landed_commit_is_a_new_nightly() {
        let latest = v("0.2.0-nightly.20260829");
        let decision = decide(Some(&latest), &commit(), &v("0.2.0"), day(2026, 8, 30)).unwrap();
        assert_eq!(decision, Decision::New(v("0.2.0-nightly.20260830")));
    }

    /// Nothing landed: the published nightly stands, under its own date. A
    /// restamp would be a different version.
    #[test]
    fn an_idle_day_is_the_current_nightly_as_published() {
        let latest = v("0.2.0-nightly.20260829");
        let decision = decide(Some(&latest), &[], &v("0.2.0"), day(2026, 8, 30)).unwrap();
        assert_eq!(decision, Decision::Current(v("0.2.0-nightly.20260829")));
    }

    /// Scenario _A second run on a taken day_: the day's nightly exists and
    /// the stores hold its build, so the work waits for tomorrow.
    #[test]
    fn a_second_run_on_a_taken_day_publishes_nothing() {
        use crate::releases::index::fixture::Repo;

        let latest = v("0.2.0-nightly.20260830");
        let decision = decide(Some(&latest), &commit(), &v("0.2.0"), day(2026, 8, 30)).unwrap();
        assert_eq!(decision, Decision::Taken(v("0.2.0-nightly.20260830")));

        let repo = Repo::new();
        repo.product("feat: the nightly's work");
        repo.tag("v0.2.0-nightly.20260830");
        repo.index("nightly", "v0.2.0-nightly.20260830");
        repo.product("fix: landed later the same day");
        let commits = repo.product_commits("v0.2.0-nightly.20260830..HEAD");
        assert_eq!(commits, ["fix: landed later the same day"]);
        let decision = decide(Some(&latest), &commits, &v("0.2.0"), day(2026, 8, 30)).unwrap();
        assert!(matches!(decision, Decision::Taken(_)), "{decision:?}");
    }

    /// A published nightly dated after today can only mean a wrong clock, and
    /// a lower version must not be minted over it.
    #[test]
    fn a_nightly_dated_after_today_is_refused() {
        let latest = v("0.2.0-nightly.20260830");
        assert!(decide(Some(&latest), &commit(), &v("0.2.0"), day(2026, 8, 29)).is_err());
    }

    /// The entry recording a nightly lands right after its tag. Were it to
    /// count as work, every quiet day would publish a nightly.
    #[test]
    fn an_index_commit_does_not_wake_a_quiet_repository() {
        use crate::releases::index::fixture::Repo;

        let latest = v("0.2.0-nightly.20260829");
        let range = "v0.2.0-nightly.20260829..HEAD";

        let repo = Repo::new();
        repo.product("feat: the nightly's work");
        repo.tag("v0.2.0-nightly.20260829");
        repo.index("nightly", "v0.2.0-nightly.20260829");

        let commits = repo.product_commits(range);
        assert!(commits.is_empty(), "{commits:?}");
        let decision = decide(Some(&latest), &commits, &v("0.2.0"), day(2026, 8, 30)).unwrap();
        assert!(matches!(decision, Decision::Current(_)), "{decision:?}");

        // ...while real work alongside it still publishes.
        repo.product("fix: something landed");
        let commits = repo.product_commits(range);
        assert_eq!(commits, ["fix: something landed"]);
        let decision = decide(Some(&latest), &commits, &v("0.2.0"), day(2026, 8, 30)).unwrap();
        assert_eq!(decision, Decision::New(v("0.2.0-nightly.20260830")));
    }

    #[test]
    fn a_new_target_has_no_nightly_yet() {
        let tags = owned(&["v0.2.0-nightly.20260829"]);
        assert_eq!(latest_nightly(&v("0.3.0"), &tags).unwrap(), None);
        let decision = decide(None, &commit(), &v("0.3.0"), day(2026, 8, 30)).unwrap();
        assert_eq!(decision, Decision::New(v("0.3.0-nightly.20260830")));
    }

    #[test]
    fn the_latest_nightly_is_the_newest_date() {
        let tags = owned(&[
            "v0.2.0-nightly.20260829",
            "v0.2.0-nightly.20260901",
            "v0.2.0-nightly.20260830",
        ]);
        let latest = latest_nightly(&v("0.2.0"), &tags).unwrap().unwrap();
        assert_eq!(latest.pre.as_str(), "nightly.20260901");
    }

    #[test]
    fn non_nightly_tags_are_ignored() {
        let tags = owned(&[
            "v0.2.0",
            "v0.2.0-pre.3",
            "v0.2.0-nightly.20260829",
            "vacation",
        ]);
        let latest = latest_nightly(&v("0.2.0"), &tags).unwrap().unwrap();
        assert_eq!(latest.pre.as_str(), "nightly.20260829");
    }

    /// Scenario _An earlier-grammar tag is refused_: a stale shape fails the
    /// planner by name instead of voting, whichever target it names.
    #[test]
    fn an_earlier_grammar_tag_fails_the_planner_by_name() {
        for stale in ["v0.1.0-nightly.2+2026.11.08", "v0.2.0-nightly.6+2026.10.21"] {
            let tags = owned(&[stale, "v0.1.0-nightly.20261109"]);
            let error = latest_nightly(&v("0.1.0"), &tags).unwrap_err().to_string();
            assert!(error.contains(stale), "{error}");
        }
    }

    /// nightly < preview < stable, and the next target outranks the release.
    #[test]
    fn channels_stay_ordered() {
        let ordered = [
            v("0.2.0-nightly"),
            v("0.2.0-nightly.20260829"),
            v("0.2.0-nightly.20260830"),
            v("0.2.0-pre.1"),
            v("0.2.0"),
            v("0.3.0-nightly.20260901"),
        ];
        let mut sorted = ordered.to_vec();
        sorted.sort();
        assert_eq!(sorted, ordered, "channel ordering is not monotonic");
    }
}
