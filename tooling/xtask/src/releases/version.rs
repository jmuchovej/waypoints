//! One reading of a version for every release task.
//!
//! A version names its own channel (a `nightly` prerelease is nightly, any
//! other prerelease a preview, none stable) and its own series (the
//! `v{major}.{minor}.x` branch every release of one minor shares). Deriving
//! both here means the uploader, the manifest writer, the index, and the
//! planners can never disagree about the same tag — and, applied to a
//! manifest rather than a tag, it is how a tree says which channel it ships
//! as. `nix/channel.nix` is the same reading for the overlay.
//!
//! A version also knows how to make each [`Transition`] of the release cycle:
//! see [`Version::transition`].

use crate::Channel;
use anyhow::{Context as _, Result, bail};
use cargo_metadata::semver::{self, BuildMetadata, Prerelease};
use clap::ValueEnum;
use derive_more::{AsMut, AsRef, Deref, From, Into};
use next_version::VersionUpdater;
use std::{fmt, str::FromStr};
use strum::{EnumIter, IntoEnumIterator as _};

/// The prerelease identifier every preview carries.
pub const PRE: &str = "pre";

/// The prerelease identifier every nightly carries.
pub const NIGHTLY: &str = "nightly";

/// The manifest whose `version` is the app's — and, by its suffix, the
/// channel the tree ships as. Tauri reads the version from here when
/// `Tauri.toml` leaves it unset.
pub const APP_MANIFEST: &str = "src-tauri/Cargo.toml";

/// The four ref-minting transitions of a release cycle, in the order a
/// version meets them.
///
/// One branch exists per minor version, `v{major}.{minor}.x`. It is cut from
/// `main` as a preview, advances through `-pre.N` while it stabilises, is
/// promoted in place to stable, and then takes patches until the next minor
/// supersedes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, ValueEnum)]
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

/// A version read the way the release tasks read one. Dereferences to the
/// [`semver::Version`] it wraps, so the semver fields stay reachable.
///
/// Two spellings parse: [`Version::parse`] takes a bare version, and
/// [`FromStr`] (so `"v0.1.1".parse()`) takes a tag, since every task that
/// reads one from the command line is handed a tag.
#[derive(Deref, Into, From, AsRef, AsMut, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(semver::Version);

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self(semver::Version::new(major, minor, patch))
    }

    /// Parses a bare version, `0.1.1`.
    pub fn parse(version: &str) -> Result<Self> {
        semver::Version::parse(version)
            .with_context(|| format!("`{version}` is not a valid version"))
            .map(Self)
    }

    /// Strips the tag's leading `v` and parses the rest — the one rule every
    /// task that reads a tag follows.
    pub fn from_tag(tag: &str) -> Result<Self> {
        let version = tag
            .strip_prefix('v')
            .with_context(|| format!("tag `{tag}` has no leading `v`"))?;
        Self::parse(version).with_context(|| format!("tag `{tag}` is not a valid version"))
    }

    /// The tag this version is published under: `v{version}`.
    pub fn tag(&self) -> String {
        format!("v{self}")
    }

    pub fn channel(&self) -> Channel {
        match self.pre.split('.').next().unwrap_or_default() {
            "" => Channel::Stable,
            NIGHTLY => Channel::Nightly,
            _ => Channel::Preview,
        }
    }

    pub fn is_stable(&self) -> bool {
        self.channel().is_stable()
    }

    pub fn is_preview(&self) -> bool {
        self.channel().is_preview()
    }

    pub fn is_nightly(&self) -> bool {
        self.channel().is_nightly()
    }

    /// The branch this version belongs to. Every patch and preview of one
    /// minor shares a branch, which is what lets a preview be promoted in
    /// place.
    pub fn release_branch(&self) -> String {
        format!("v{}.{}.x", self.major, self.minor)
    }

    /// The release store's series prefix: the same `v{major}.{minor}.x` as
    /// [`release_branch`](Self::release_branch), under the name the store's
    /// key layout uses for it. One minor line holds every channel's versions,
    /// which is why the channel still has to lead the key — see the design's
    /// "Channel leads the key" decision.
    pub fn series(&self) -> String {
        self.release_branch()
    }

    /// The release this version is a view of: the version with any suffix
    /// stripped.
    pub fn target(&self) -> Self {
        Self::new(self.major, self.minor, self.patch)
    }

    /// The version `transition` leads to from this one.
    ///
    /// Only a cut needs anything beyond the version: it is measured from the
    /// last stable release, and `commits` — the work since it — decide the
    /// target, because `main`'s own version is provisional. The other three
    /// are mechanical and ignore `commits`: a release branch carries
    /// cherry-picks whose messages describe the original change rather than
    /// its severity as a backport.
    pub fn transition(&self, transition: Transition, commits: &[String]) -> Result<Self> {
        match transition {
            Transition::Cut => self.cut(commits),
            Transition::Advance => self.advance(),
            Transition::Promote => self.promote(),
            Transition::Patch => self.patch(),
        }
    }

    /// `0.1.0` → `0.2.0-pre.1` after a feature, `0.1.1-pre.1` after fixes
    /// only: the first candidate for the release `commits` add up to. `self`
    /// is the last stable release — `0.0.0` before there is one.
    pub fn cut(&self, commits: &[String]) -> Result<Self> {
        let mut target = VersionUpdater::new()
            .with_features_always_increment_minor(true)
            .increment(self, commits);
        target.pre = Prerelease::new(&format!("{PRE}.1"))
            .with_context(|| format!("`{PRE}.1` is not a valid prerelease"))?;
        Ok(Self(target))
    }

    /// `0.2.0-pre.1` → `0.2.0-pre.2`: [`next_preview`](Self::next_preview),
    /// refused on anything but a preview.
    pub fn advance(&self) -> Result<Self> {
        if !self.is_preview() {
            bail!("{self} is not a preview; `advance` only applies to one");
        }
        self.next_preview()
    }

    /// `0.2.0-pre.7` → `0.2.0`. The branch keeps its identity; only its role
    /// moves.
    pub fn promote(&self) -> Result<Self> {
        if self.is_stable() {
            bail!("{self} is already stable; nothing to promote");
        }
        let mut stable = self.0.clone();
        stable.pre = Prerelease::EMPTY;
        Ok(Self(stable))
    }

    /// `0.2.0` → `0.2.1`, for a stable branch taking a fix.
    pub fn patch(&self) -> Result<Self> {
        if !self.is_stable() {
            bail!("{self} is a preview; use `advance` instead of `patch`");
        }
        Ok(Self::new(self.major, self.minor, self.patch + 1))
    }

    /// Where `main` goes after cutting toward this target: provisionally the
    /// next minor, corrected by the next cut's commit history if the cycle
    /// turns out smaller. It carries the `nightly` suffix, which is how
    /// `main` names its channel; the nightly planner strips it to find the
    /// target.
    pub fn next_main(&self) -> Result<Self> {
        let mut next = semver::Version::new(self.major, self.minor + 1, 0);
        next.pre = Prerelease::new(NIGHTLY)
            .with_context(|| format!("`{NIGHTLY}` is not a valid prerelease"))?;
        Ok(Self(next))
    }

    /// `0.2.0-pre.1` → `0.2.0-pre.2`, `0.2.0-pre` → `0.2.0-pre.1`.
    ///
    /// Counts the trailing identifier directly rather than going through a
    /// commit-driven bumper: a backport's message describes the original
    /// change rather than its severity here, so advancing a preview is
    /// mechanical by design. Contrast [`next_nightly`](Self::next_nightly),
    /// where the commits are the whole point.
    pub fn next_preview(&self) -> Result<Self> {
        let pre = self.pre.as_str();
        let (stem, index) = match pre.rsplit_once('.') {
            Some((stem, index)) => match index.parse::<u64>() {
                Ok(index) => (stem, index),
                // A non-numeric tail is part of the name, not a counter.
                Err(_) => (pre, 0),
            },
            None => (pre, 0),
        };

        let mut next = self.0.clone();
        next.pre = Prerelease::new(&format!("{stem}.{}", index + 1))
            .with_context(|| format!("`{stem}.{}` is not a valid prerelease", index + 1))?;
        Ok(Self(next))
    }

    /// The first nightly of the release this version is a view of:
    /// `{target}-nightly.1`.
    pub fn first_nightly(&self) -> Result<Self> {
        let mut version: semver::Version = self.target().into();
        version.pre = Prerelease::new(&format!("{NIGHTLY}.1"))
            .with_context(|| format!("`{NIGHTLY}.1` is not a valid prerelease"))?;
        Ok(Self(version))
    }

    /// `0.2.0-nightly.3` → `0.2.0-nightly.4` when `commits` landed since
    /// it, else `None`.
    ///
    /// `N` counts the nightlies shipped since the target last changed, and
    /// is bumped by [`VersionUpdater`], which increments a trailing numeric
    /// prerelease identifier and leaves build metadata alone — and,
    /// usefully, returns the version *unchanged* when no commits landed, so
    /// "is there anything new?" and "what is the next version?" are one
    /// call. The date is the caller's to stamp: see
    /// [`stamped`](Self::stamped).
    pub fn next_nightly(&self, commits: &[String]) -> Result<Option<Self>> {
        if !self.is_nightly() {
            bail!("{self} is not a nightly; only a nightly has a next nightly");
        }
        let bumped = VersionUpdater::new().increment(self, commits);
        Ok((bumped.pre != self.pre).then_some(Self(bumped)))
    }

    /// This version carrying `stamp` as its build metadata, replacing
    /// whatever came through — a preserved date is a stale one.
    pub fn stamped(&self, stamp: &str) -> Result<Self> {
        let mut version = self.0.clone();
        version.build = BuildMetadata::new(stamp)
            .with_context(|| format!("\"{stamp}\" is not valid build metadata"))?;
        Ok(Self(version))
    }

    /// The store build numbers a nightly's date stamp yields, as
    /// `(android, ios)`.
    ///
    /// Both are the date: Android's `versionCode` is a single integer, so the
    /// separators are dropped; iOS's `CFBundleVersion` takes period-separated
    /// components and keeps them. Both increase monotonically because dates
    /// do, and 8 digits stays far under Android's 2_100_000_000 ceiling.
    ///
    /// ```text
    /// 0.2.0-nightly.1+2026.08.29  ->  20260829   2026.08.29
    /// 0.2.0-nightly.2+2026.08.30  ->  20260830   2026.08.30
    /// ```
    pub fn build_numbers(&self) -> Result<(String, String)> {
        let stamp = self.build.as_str();
        if stamp.is_empty() {
            bail!("{self} carries no date; a nightly needs one for its build number");
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
}

/// Parses a tag, `v{version}`: see [`Version::from_tag`].
impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(tag: &str) -> Result<Self> {
        Self::from_tag(tag)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }
    fn msgs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_version_names_its_own_channel() {
        assert_eq!(v("0.2.0").channel(), Channel::Stable);
        assert_eq!(v("0.2.1").channel(), Channel::Stable);
        assert_eq!(v("0.2.0-pre.1").channel(), Channel::Preview);
        assert_eq!(v("0.2.0-pre").channel(), Channel::Preview);
        assert_eq!(v("0.2.0-nightly.7+2026.08.29").channel(), Channel::Nightly);
    }

    #[test]
    fn one_branch_serves_every_release_of_a_minor() {
        for version in ["0.2.0-pre.1", "0.2.0", "0.2.7"] {
            assert_eq!(v(version).release_branch(), "v0.2.x");
            assert_eq!(v(version).series(), "v0.2.x");
        }
    }

    #[test]
    fn a_tag_derives_its_own_channel_and_series() {
        let preview: Version = "v0.2.0-pre.1".parse().unwrap();
        assert_eq!(preview.channel(), Channel::Preview);
        let stable: Version = "v0.1.1".parse().unwrap();
        assert_eq!(stable.channel(), Channel::Stable);
        assert_eq!(stable.series(), "v0.1.x");
        assert_eq!(stable.tag(), "v0.1.1");
    }

    /// A tag has a `v`; a bare version has none. Each parser refuses the
    /// other's spelling, so a task cannot read one as the other by accident.
    #[test]
    fn a_tag_and_a_bare_version_are_parsed_apart() {
        assert!(Version::from_tag("0.1.1").is_err());
        assert!("0.1.1".parse::<Version>().is_err());
        assert!(Version::parse("v0.1.1").is_err());
        assert_eq!(Version::parse("0.1.1").unwrap(), Version::new(0, 1, 1));
    }

    #[test]
    fn the_target_strips_every_suffix() {
        assert_eq!(
            v("0.2.0-nightly.7+2026.08.29").target(),
            Version::new(0, 2, 0)
        );
        assert_eq!(v("0.2.0-pre.3").target(), Version::new(0, 2, 0));
    }

    /// Every transition reached through the one dispatch, so a planner
    /// cannot pair a transition with the wrong arithmetic.
    #[test]
    fn a_version_makes_each_transition() {
        let next = |from: &str, transition, commits: &[&str]| {
            v(from)
                .transition(transition, &msgs(commits))
                .unwrap()
                .to_string()
        };
        assert_eq!(
            next("0.1.0", Transition::Cut, &["feat: charts"]),
            "0.2.0-pre.1"
        );
        assert_eq!(next("0.2.0-pre.1", Transition::Advance, &[]), "0.2.0-pre.2");
        assert_eq!(next("0.2.0-pre.7", Transition::Promote, &[]), "0.2.0");
        assert_eq!(next("0.2.0", Transition::Patch, &[]), "0.2.1");
    }

    /// The commits decide what a cut is heading toward, not the version it
    /// is measured from.
    #[test]
    fn a_cut_is_sized_by_its_commits() {
        assert_eq!(
            v("0.1.0")
                .cut(&msgs(&["feat: charts", "fix: typo"]))
                .unwrap(),
            v("0.2.0-pre.1")
        );
        assert_eq!(
            v("0.1.0").cut(&msgs(&["fix: a bug"])).unwrap(),
            v("0.1.1-pre.1")
        );
        // Before any release, the first cut is measured from nothing.
        assert_eq!(
            v("0.0.0").cut(&msgs(&["feat: first"])).unwrap(),
            v("0.1.0-pre.1")
        );
    }

    #[test]
    fn advancing_a_preview_counts_up() {
        assert_eq!(v("0.2.0-pre.1").next_preview().unwrap(), v("0.2.0-pre.2"));
        assert_eq!(v("0.2.0-pre.1").advance().unwrap(), v("0.2.0-pre.2"));
        // Numeric, not lexical: the tenth candidate must outrank the ninth.
        assert_eq!(v("0.2.0-pre.9").advance().unwrap(), v("0.2.0-pre.10"));
        assert!(v("0.2.0-pre.9") < v("0.2.0-pre.10"));
    }

    #[test]
    fn an_uncounted_preview_gains_a_counter() {
        assert_eq!(v("0.2.0-pre").advance().unwrap(), v("0.2.0-pre.1"));
    }

    /// Promotion keeps the branch; patching stays on it.
    #[test]
    fn promotion_and_patching_stay_on_the_branch() {
        let promoted = v("0.2.0-pre.7").promote().unwrap();
        assert_eq!(promoted, v("0.2.0"));
        assert_eq!(promoted.release_branch(), "v0.2.x");

        let patched = v("0.2.0").patch().unwrap();
        assert_eq!(patched, v("0.2.1"));
        assert_eq!(patched.release_branch(), "v0.2.x");
    }

    #[test]
    fn transitions_refuse_the_wrong_channel() {
        assert!(v("0.2.0").advance().is_err());
        assert!(v("0.2.0-nightly.3+2026.08.29").advance().is_err());
        assert!(v("0.2.0").promote().is_err());
        assert!(v("0.2.0-pre.1").patch().is_err());
        assert!(v("0.2.0-pre.1").next_nightly(&[]).is_err());
    }

    #[test]
    fn the_first_nightly_of_a_target_starts_at_one() {
        assert_eq!(v("0.2.0").first_nightly().unwrap(), v("0.2.0-nightly.1"));
        // ...from whatever view of the target it is asked of.
        assert_eq!(
            v("0.3.0-nightly").first_nightly().unwrap(),
            v("0.3.0-nightly.1")
        );
    }

    /// The bump and the "anything new?" check are the same call: a landed
    /// commit advances the counter, an idle day yields nothing.
    #[test]
    fn a_nightly_advances_only_when_something_landed() {
        let latest = v("0.2.0-nightly.1+2026.08.29");
        let next = latest
            .next_nightly(&msgs(&["fix: something landed"]))
            .unwrap();
        assert_eq!(next, Some(v("0.2.0-nightly.2+2026.08.29")));
        assert_eq!(latest.next_nightly(&[]).unwrap(), None);
    }

    /// `main` names its channel the way every branch does — in its version.
    /// The suffix must sort below everything the line publishes and below
    /// the series it cuts, or the staircase breaks.
    #[test]
    fn main_targets_the_next_minor_as_nightly() {
        let next = v("0.2.0").next_main().unwrap();
        assert_eq!(next.to_string(), "0.3.0-nightly");
        assert_eq!(next.channel(), Channel::Nightly);
        assert!(next < v("0.3.0-nightly.1+2026.08.29"));
        assert!(v("0.3.0-nightly.1+2026.08.29") < v("0.3.0-pre.1"));
        assert!(v("0.3.0-pre.1") < v("0.3.0"));
    }

    #[test]
    fn build_numbers_come_from_the_date() {
        let (android, ios) = v("0.2.0-nightly.7+2026.08.29").build_numbers().unwrap();
        assert_eq!((android.as_str(), ios.as_str()), ("20260829", "2026.08.29"));
    }

    /// Android's `versionCode` is one integer and must only ever climb.
    #[test]
    fn version_codes_increase_with_the_date() {
        let earlier = v("0.2.0-nightly.9+2026.08.29").build_numbers().unwrap().0;
        let later = v("0.2.0-nightly.1+2026.09.01").build_numbers().unwrap().0;
        assert!(earlier.parse::<u64>().unwrap() < later.parse::<u64>().unwrap());
        // ...even across a target bump, which restarts N but not the date.
        assert!(later.parse::<u64>().unwrap() < 2_100_000_000);
    }

    #[test]
    fn a_nightly_without_a_date_is_rejected() {
        assert!(v("0.2.0-nightly.1").build_numbers().is_err());
    }

    /// A preserved date would be the previous run's.
    #[test]
    fn stamping_replaces_the_build_metadata() {
        let restamped = v("0.2.0-nightly.1+2026.08.29")
            .stamped("2026.09.01")
            .unwrap();
        assert_eq!(restamped.to_string(), "0.2.0-nightly.1+2026.09.01");
    }
}
