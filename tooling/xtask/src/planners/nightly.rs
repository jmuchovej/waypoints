//! Decides what a Nightly workflow run builds, and whether it publishes.
//!
//! A nightly is `{target}-nightly.{N}+{YYYY.MM.DD}`. `N` counts the nightlies
//! shipped since the target last changed, and carries the semver ordering;
//! the date is build metadata, which semver ignores for precedence, so it is
//! free to be documentation that the store build numbers are also read from.
//! How `N` moves is [`Version::next_nightly`]'s; this module supplies what it
//! is applied to — the newest published nightly and the commits since it —
//! and stamps the date over whatever came through, because a preserved date
//! is a stale one.
//!
//! The date being the store build number is also the rate limit: two runs on
//! one day would produce the same `versionCode`, so a nightly ships at most
//! once a day. A manual dispatch therefore builds without publishing — but it
//! still resolves the version and build numbers the scheduled run would use,
//! so its artifacts are stamped like real nightlies and can be uploaded by
//! hand for testing.
//!
//! Tags are read from the checkout with `git` and created through the API,
//! so the job needs neither deep history nor a committer identity.

use crate::{
    releases::{index, version::Version},
    tools::git,
};
use anyhow::Result;
use chrono::{DateTime, Utc};

/// What the planner decided, and what the workflow reads back as job outputs.
///
/// Serialized field-for-field into `$GITHUB_OUTPUT`, so these field names are
/// the output keys.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct Plan {
    /// Empty on a build-only run.
    pub tag: String,
    /// Whether this run itself created `tag`'s ref — true only on the
    /// publish path, which always creates the ref right after. A withdraw
    /// job reads this back to decide whether it may delete the tag: this
    /// run's own ref, or one that predates it (a tag push, for the channels
    /// that have no planner at all).
    pub tag_minted: bool,
    pub should_build: bool,
    /// The full nightly version, `{target}-nightly.{N}+{date}`, which the
    /// builds stamp into the app. Empty only when nothing builds.
    pub version: String,
    /// Android `versionCode`. Empty when `version` is.
    pub version_code: String,
    /// iOS `CFBundleVersion`. Empty when `version` is.
    pub bundle_version: String,
}

impl Plan {
    /// Derives the build numbers from the version, so the two cannot disagree.
    pub fn publish(version: &Version) -> Result<Self> {
        Ok(Self {
            tag: version.tag(),
            tag_minted: true,
            ..Self::build_only(version)?
        })
    }

    /// Builds `version` without minting its tag — a manual dispatch. The
    /// artifacts are stamped exactly as the scheduled run would stamp them.
    pub fn build_only(version: &Version) -> Result<Self> {
        let (version_code, bundle_version) = version.build_numbers()?;
        Ok(Self {
            tag: String::new(),
            tag_minted: false,
            should_build: true,
            version: version.to_string(),
            version_code,
            bundle_version,
        })
    }

    pub const SKIP: Self = Self {
        tag: String::new(),
        tag_minted: false,
        should_build: false,
        version: String::new(),
        version_code: String::new(),
        bundle_version: String::new(),
    };
}

/// What a run builds, given what is already published and what landed.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Something landed, or nothing is published yet: the next nightly,
    /// stamped with today's date. A scheduled run publishes this.
    New(Version),
    /// Nothing landed: the current nightly restamped with today's date — a
    /// rebuild, not a new one. A scheduled run publishes nothing; a
    /// build-only dispatch builds it anyway.
    Current(Version),
}

impl Decision {
    /// What the run's artifacts are stamped with, either way.
    pub fn version(&self) -> &Version {
        match self {
            Decision::New(version) | Decision::Current(version) => version,
        }
    }
}

/// Decides from `latest` (the newest nightly published for `target`) and
/// `commits` (the work since it), stamping the answer with `stamp`.
pub fn decide(
    latest: Option<&Version>,
    commits: &[String],
    target: &Version,
    stamp: &str,
) -> Result<Decision> {
    let Some(latest) = latest else {
        return Ok(Decision::New(target.first_nightly()?.stamped(stamp)?));
    };
    Ok(match latest.next_nightly(commits)? {
        Some(next) => Decision::New(next.stamped(stamp)?),
        None => Decision::Current(latest.stamped(stamp)?),
    })
}

/// Today's UTC date, as the build metadata a nightly carries.
pub fn date_stamp(now: DateTime<Utc>) -> String {
    now.format("%Y.%m.%d").to_string()
}

/// The newest nightly already published for `target`.
pub fn latest_nightly(target: &Version, tags: &[String]) -> Option<Version> {
    tags.iter()
        .filter_map(|tag| tag.parse::<Version>().ok())
        .filter(|version| version.is_nightly() && version.target() == *target)
        .max()
}

pub fn nightly_tags() -> Result<Vec<String>> {
    Ok(git::run(&["tag", "--list", "v*-nightly.*"])?
        .lines()
        .map(str::to_owned)
        .collect())
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
    /// (`Plan::publish`, immediately followed by the tag creation in the
    /// task) — a withdraw job reads this back to know whether it may delete
    /// the tag.
    #[test]
    #[allow(
        clippy::assertions_on_constants,
        reason = "SKIP is const, but Plan isn't const-droppable, so `const { assert!(..) }` fails to compile (rust-lang/rust#133214)"
    )]
    fn tag_minted_is_set_only_on_the_publish_path() {
        let version = v("0.2.0-nightly.7+2026.08.29");
        assert!(Plan::publish(&version).unwrap().tag_minted);
        assert!(!Plan::build_only(&version).unwrap().tag_minted);
        assert!(!Plan::SKIP.tag_minted);
    }

    #[test]
    fn the_first_nightly_of_a_target_is_new() {
        let decision = decide(None, &[], &v("0.2.0"), "2026.08.29").unwrap();
        assert_eq!(decision, Decision::New(v("0.2.0-nightly.1+2026.08.29")));
    }

    #[test]
    fn a_landed_commit_is_a_new_nightly() {
        let latest = v("0.2.0-nightly.1+2026.08.29");
        let decision = decide(Some(&latest), &commit(), &v("0.2.0"), "2026.08.30").unwrap();
        assert_eq!(decision, Decision::New(v("0.2.0-nightly.2+2026.08.30")));
    }

    /// Nothing landed: the current nightly under today's date, so a dispatch
    /// rebuilds it rather than inventing a counter the scheduled run would
    /// not — and the scheduled run reads it as nothing to publish.
    #[test]
    fn an_idle_day_is_the_current_nightly_restamped() {
        let latest = v("0.2.0-nightly.3+2026.08.29");
        let decision = decide(Some(&latest), &[], &v("0.2.0"), "2026.08.30").unwrap();
        assert_eq!(decision, Decision::Current(v("0.2.0-nightly.3+2026.08.30")));
        assert_eq!(decision.version().to_string(), "0.2.0-nightly.3+2026.08.30");
    }

    /// The entry recording a nightly lands right after its tag. Were it to
    /// count as work, every quiet day would publish a nightly.
    #[test]
    fn an_index_commit_does_not_wake_a_quiet_repository() {
        use crate::releases::index::fixture::Repo;

        let latest = v("0.2.0-nightly.1+2026.08.29");
        let range = "v0.2.0-nightly.1+2026.08.29..HEAD";

        let repo = Repo::new();
        repo.product("feat: the nightly's work");
        repo.tag("v0.2.0-nightly.1+2026.08.29");
        repo.index("nightly", "v0.2.0-nightly.1+2026.08.29");

        let commits = repo.product_commits(range);
        assert!(commits.is_empty(), "{commits:?}");
        let decision = decide(Some(&latest), &commits, &v("0.2.0"), "2026.08.30").unwrap();
        assert!(matches!(decision, Decision::Current(_)), "{decision:?}");

        // ...while real work alongside it still publishes.
        repo.product("fix: something landed");
        let commits = repo.product_commits(range);
        assert_eq!(commits, ["fix: something landed"]);
        let decision = decide(Some(&latest), &commits, &v("0.2.0"), "2026.08.30").unwrap();
        assert_eq!(decision, Decision::New(v("0.2.0-nightly.2+2026.08.30")));
    }

    /// A preserved date would be the previous run's.
    #[test]
    fn the_date_is_restamped_not_carried() {
        let latest = v("0.2.0-nightly.1+2026.08.29");
        for decision in [
            decide(Some(&latest), &commit(), &v("0.2.0"), "2026.09.01").unwrap(),
            decide(Some(&latest), &[], &v("0.2.0"), "2026.09.01").unwrap(),
        ] {
            assert_eq!(decision.version().build.as_str(), "2026.09.01");
        }
    }

    #[test]
    fn the_counter_restarts_when_the_target_moves() {
        let tags = owned(&["v0.2.0-nightly.9+2026.08.29"]);
        assert_eq!(latest_nightly(&v("0.3.0"), &tags), None);
        let decision = decide(None, &commit(), &v("0.3.0"), "2026.08.30").unwrap();
        assert_eq!(decision, Decision::New(v("0.3.0-nightly.1+2026.08.30")));
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
        use crate::workflows::{WorkflowContext, jobs};
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
