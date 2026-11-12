// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plans a channel's release run: what it builds, whether it publishes, and —
//! for nightly — the tag it mints. The deciding is
//! [`xtask::planners::nightly`]'s; this reads the checkout and the flags,
//! decides, and publishes the plan as the step's outputs and summary.

use anyhow::{Result, bail};
use chrono::Utc;
use clap::Parser;
use xtask::{
    Channel,
    planners::nightly::{self, Decision, Plan},
    release_groups,
    releases::version::Version,
    tools::{git, github},
    workspace,
};

#[derive(Parser)]
pub struct Options {
    /// The channel this run publishes.
    channel: Channel,
    /// The existing tag a preview or stable run publishes. Nightly mints its
    /// own; a dry run left without one uses the manifest's version.
    #[arg(long)]
    tag: Option<String>,
    /// Decide exactly as a real run would, mint nothing, and report the tag
    /// and version it would have minted. Every write then goes to the run's
    /// own artifacts.
    #[arg(long)]
    dry_run: bool,
    /// Run the test and build stages: the default.
    #[arg(long, conflicts_with = "no_build")]
    build: bool,
    /// Decide and report only. Needs `--dry-run`: a real run with no builds
    /// would mint a tag with no artifacts behind it.
    #[arg(long)]
    no_build: bool,
}

pub fn run(args: Options) -> Result<()> {
    let build = builds(args.dry_run, args.no_build)?;
    let tag = args.tag.filter(|tag| !tag.is_empty());

    release_groups::load(&workspace::load()?)?;
    let manifest = Version::from(workspace::version()?);
    if manifest.channel() != args.channel {
        bail!(
            "the manifest declares {manifest}, a {} version, and this run is for the {} channel",
            manifest.channel(),
            args.channel
        );
    }

    let (plan, report) = match args.channel {
        Channel::Nightly => plan_nightly(&manifest, tag.as_deref(), args.dry_run, build)?,
        Channel::Preview | Channel::Stable => {
            plan_release(args.channel, &manifest, tag.as_deref(), args.dry_run, build)?
        }
    };
    println!("{report}");
    github::add_summary(&summary(&report, &plan))?;
    github::set_outputs(&plan)
}

/// Whether the stages run. Refused before anything is read: a real run with
/// no builds would mint a tag with no artifacts behind it.
fn builds(dry_run: bool, no_build: bool) -> Result<bool> {
    if no_build && !dry_run {
        bail!(
            "`--no-build` needs `--dry-run`: a real run with no builds would mint a tag with no artifacts behind it"
        );
    }
    Ok(!no_build)
}

fn plan_nightly(
    manifest: &Version,
    tag: Option<&str>,
    dry_run: bool,
    build: bool,
) -> Result<(Plan, String)> {
    if let Some(tag) = tag {
        bail!("nightly mints its own tag; `--tag {tag}` applies to preview and stable");
    }
    let target = manifest.target();
    let tags = nightly::nightly_tags()?;
    let latest = nightly::latest_nightly(&target, &tags)?;
    let commits = nightly::commits_since(latest.as_ref())?;
    let today = Utc::now().date_naive();
    let decision = nightly::decide(latest.as_ref(), &commits, &target, today)?;

    let verdict = match &decision {
        Decision::New(version) => format!("Work landed since the last nightly; {version} is next."),
        Decision::Current(version) => {
            format!("Nothing landed since the last nightly {version}; nothing new to publish.")
        }
        Decision::Taken(version) => format!(
            "Today's nightly {version} already exists; the work since it ships with the next scheduled nightly."
        ),
    };

    if dry_run {
        let todays = target.nightly(today)?;
        let plan = Plan::rehearsal(&todays, build)?;
        let rehearsing = rehearsing("mint", &todays, build);
        return Ok((plan, format!("Dry run: {verdict} {rehearsing}")));
    }
    match decision {
        Decision::New(version) => {
            let head = git::run(&["rev-parse", "HEAD"])?;
            let plan = Plan::publish(&version)?;
            github::create_tag(&plan.tag, &head)?;
            let report = format!("Tagged nightly {tag} at {head}.", tag = plan.tag);
            Ok((plan, report))
        }
        Decision::Current(_) | Decision::Taken(_) => Ok((Plan::SKIP, verdict)),
    }
}

fn plan_release(
    channel: Channel,
    manifest: &Version,
    tag: Option<&str>,
    dry_run: bool,
    build: bool,
) -> Result<(Plan, String)> {
    let version = match tag {
        Some(tag) => {
            let version: Version = tag.parse()?;
            if version.channel() != channel {
                bail!(
                    "tag {tag} names the {} channel, and this run is for {channel}",
                    version.channel()
                );
            }
            version
        }
        None if dry_run => manifest.clone(),
        None => bail!(
            "a real {channel} run names the tag it publishes: pass `--tag`, the pushed tag or an existing one to publish again"
        ),
    };
    if dry_run {
        let plan = Plan::rehearsal(&version, build)?;
        let rehearsing = rehearsing("publish", &version, build);
        return Ok((plan, format!("Dry run: {rehearsing}")));
    }
    Ok((
        Plan::release(&version)?,
        format!("Publishing {}.", version.tag()),
    ))
}

fn rehearsing(verb: &str, version: &Version, build: bool) -> String {
    let tag = version.tag();
    if build {
        format!("Would {verb} {tag}; building {version} and publishing nothing.")
    } else {
        format!("Would {verb} {tag}; building and publishing nothing.")
    }
}

/// The job summary: the verdict, and the outputs a reader would otherwise dig
/// out of the log.
fn summary(report: &str, plan: &Plan) -> String {
    format!(
        "### Plan\n\n{report}\n\n| output | value |\n| --- | --- |\n| tag | `{tag}` |\n| version | `{version}` |\n| dry_run | {dry_run} |\n| should_build | {should_build} |\n",
        tag = plan.tag,
        version = plan.version,
        dry_run = plan.dry_run,
        should_build = plan.should_build,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario _No-build without dry-run is refused_.
    #[test]
    fn no_build_needs_dry_run() {
        assert!(builds(false, true).is_err());
        assert!(!builds(true, true).unwrap());
        assert!(builds(true, false).unwrap());
        assert!(builds(false, false).unwrap());
    }

    #[test]
    fn the_cli_takes_a_channel_and_the_flags() {
        let options =
            Options::try_parse_from(["publish-plan", "preview", "--tag", "v0.2.0-pre.1"]).unwrap();
        assert_eq!(options.channel, Channel::Preview);
        assert_eq!(options.tag.as_deref(), Some("v0.2.0-pre.1"));
        assert!(!options.dry_run && !options.no_build);

        let options =
            Options::try_parse_from(["publish-plan", "nightly", "--dry-run", "--no-build"])
                .unwrap();
        assert_eq!(options.channel, Channel::Nightly);
        assert!(options.dry_run && options.no_build);

        assert!(
            Options::try_parse_from(["publish-plan", "nightly", "--build", "--no-build"]).is_err()
        );
        assert!(Options::try_parse_from(["publish-plan", "weekly"]).is_err());
    }
}
