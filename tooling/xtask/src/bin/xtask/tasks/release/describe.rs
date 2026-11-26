// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `release describe --ref <ref>`: a build's identity from a ref. A tag names
//! its version, and the tree it points at has to name the same channel; any
//! other ref describes the tree itself — the app version its manifest
//! declares, dated today when it is a nightly, so a rehearsal from `main`
//! describes the nightly it would be. The outputs are the ones `prepare`
//! publishes for a build today.

use anyhow::Result;
use chrono::{NaiveDate, Utc};
use clap::Parser;
use serde::Serialize;
use xtask::{
    releases::{index, version::Version},
    tools::github,
};

#[derive(Parser)]
pub struct Options {
    /// The ref to describe: a tag, or any commit-ish for the build that
    /// would be made from its tree.
    #[arg(long = "ref")]
    reference: String,
}

/// What a build stamps, as the job's outputs.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Description {
    pub tag: String,
    pub version: String,
    pub channel: String,
    pub flavor: String,
    /// Android's `versionCode`: a nightly's date, and empty off nightly.
    pub version_code: String,
    /// iOS's `CFBundleVersion`, as `version_code`.
    pub bundle_version: String,
}

pub fn run(args: Options) -> Result<()> {
    let (_, manifest) = index::manifest_at(&args.reference)?;
    let described = describe(&args.reference, &manifest, Utc::now().date_naive())?;
    println!(
        "{} is {} on {}, built as the {} flavor",
        args.reference, described.version, described.channel, described.flavor
    );
    github::set_outputs_if_available(&described)
}

/// The identity of the build `reference` names, given the app manifest in
/// its tree. A reference that parses as a tag is one, and its tree has to
/// name the same channel; any other reference describes the tree itself —
/// the manifest's version, dated `today` when it is a nightly.
fn describe(reference: &str, manifest: &str, today: NaiveDate) -> Result<Description> {
    let version = match Version::from_tag(reference) {
        Ok(tagged) => {
            index::classify(reference, &tagged, manifest)?;
            tagged
        }
        Err(_) => {
            let declared = index::manifest_version(manifest)?;
            if declared.is_nightly() {
                declared.nightly(today)?
            } else {
                declared
            }
        }
    };
    let channel = version.channel();
    let (version_code, bundle_version) = if version.is_nightly() {
        version.build_numbers()?
    } else {
        (String::new(), String::new())
    };
    Ok(Description {
        tag: version.tag(),
        version: version.to_string(),
        channel: channel.to_string(),
        flavor: channel.flavor(),
        version_code,
        bundle_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str) -> String {
        format!("[package]\nname = \"Waypoints\"\nversion = \"{version}\"\n")
    }

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    /// A tag minted on the wrong branch names a channel its tree does not.
    #[test]
    fn a_tag_whose_tree_names_another_channel_is_refused() {
        let error = describe(
            "v0.2.0-pre.1",
            &manifest("0.2.0-nightly"),
            day(2026, 11, 25),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("v0.2.0-pre.1"), "{error}");
        assert!(error.contains("nightly"), "{error}");
    }

    /// A tag is its own version, build numbers included for a nightly.
    #[test]
    fn a_tag_describes_itself() {
        let described = describe(
            "v0.2.0-nightly.20261124",
            &manifest("0.2.0-nightly"),
            day(2026, 11, 25),
        )
        .unwrap();
        assert_eq!(described.version, "0.2.0-nightly.20261124");
        assert_eq!(described.channel, "nightly");
        assert_eq!(described.version_code, "20261124");
        assert_eq!(described.bundle_version, "2026.11.24");
    }

    /// A rehearsal from `main` is the nightly it would be, dated today.
    #[test]
    fn a_nightly_tree_is_described_as_today_s_nightly() {
        let described = describe("main", &manifest("0.2.0-nightly"), day(2026, 11, 25)).unwrap();
        assert_eq!(described.tag, "v0.2.0-nightly.20261125");
        assert_eq!(described.version, "0.2.0-nightly.20261125");
        assert_eq!(described.channel, "nightly");
        assert_eq!(described.version_code, "20261125");
        assert_eq!(described.bundle_version, "2026.11.25");
    }

    /// Off nightly the tree's version is the build's as declared, and the
    /// store build numbers are not this command's to fill.
    #[test]
    fn a_preview_tree_keeps_its_declared_version() {
        let described = describe("v0.2.x", &manifest("0.2.0-pre.1"), day(2026, 11, 25)).unwrap();
        assert_eq!(described.tag, "v0.2.0-pre.1");
        assert_eq!(described.version, "0.2.0-pre.1");
        assert_eq!(described.channel, "preview");
        assert_eq!(described.flavor, "preview");
        assert_eq!(described.version_code, "");
        assert_eq!(described.bundle_version, "");
    }
}
