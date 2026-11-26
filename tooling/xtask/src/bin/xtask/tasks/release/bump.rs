// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `release bump`: the one planner a person or a schedule invokes, asked for
//! a channel or a library group and never a transition. For a channel it
//! reads the open series off the remote and infers the transition — a
//! preview advances a series in preview or cuts one from `main`, a stable
//! promotes or patches, nightly mints the day's nightly — refusing to move a
//! tag over bookkeeping alone, and refusing everything while a withdrawal is
//! in flight. For a group it is today's group-scoped cut. Run from `main`;
//! see [`xtask::planners::release`].

use anyhow::{Context as _, Result, bail};
use chrono::Utc;
use clap::{ArgGroup, Parser, ValueEnum as _};
use indoc::formatdoc;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use xtask::{
    Channel,
    constants::MAIN,
    planners::{
        nightly::{self, Decision, Plan},
        release as planner,
    },
    release_groups,
    releases::version::{Transition, Version},
    tools::{git, github},
    workspace,
};

#[derive(Parser)]
#[command(group = ArgGroup::new("target").required(true))]
pub struct Options {
    /// The channel to bump: nightly mints the day's nightly, preview
    /// advances the open series or cuts the next, stable promotes or patches.
    #[arg(long, group = "target")]
    channel: Option<Channel>,
    /// Cut one library group alone: mint its tag on `main`, open no series,
    /// and move nothing else.
    #[arg(long, group = "target")]
    group: Option<String>,
    /// Cut toward the app's next major; refused where the bump would not cut.
    // `requires = "channel"` would not refuse `--group x --major`: clap
    // excuses a missing required argument that conflicts with a present one,
    // and the members of a non-multiple group conflict. The conflict is what
    // holds, with the required group forcing the other flag.
    #[arg(long, conflicts_with = "group")]
    major: bool,
    /// Raise a group's bump to at least this; never lowers it.
    #[arg(long, value_enum, conflicts_with = "channel")]
    floor: Option<planner::Floor>,
    /// Plan and print without creating a ref or writing a file.
    #[arg(long)]
    dry_run: bool,
    /// The directory a dry run renders into: the version writes as patches,
    /// the refs and pull requests it would open, and the notes for the tag.
    #[arg(long, requires = "dry_run")]
    into: Option<PathBuf>,
}

/// What a later step branches on.
#[derive(Serialize)]
struct Inferred {
    transition: String,
}

pub fn run(args: Options) -> Result<()> {
    let metadata = workspace::load()?;
    let groups = release_groups::load(&metadata)?;
    let root = metadata.workspace_root.as_std_path();

    // The remote's view, which is what is published: its tags with the
    // commits they name, each open series with the version it declares, and
    // `main`'s. Nothing is fetched into the local tags, which may hold a
    // name the remote has moved.
    let tagged = git::remote_tags_at("v*")?;
    let tags: Vec<String> = tagged.iter().map(|(name, _)| name.clone()).collect();
    let series = planner::open_series_from_remote()?;
    let main_declared = planner::declared_on(MAIN)?;
    let last_stable = planner::last_stable(&tags);
    if let Some(in_flight) =
        planner::withdrawal_in_flight(&series, &tags, &main_declared, last_stable.as_ref())?
    {
        let revert = match planner::open_revert(&in_flight)? {
            Some(number) => format!("its revert is pull request #{number}"),
            None => format!(
                "its revert has not been opened; `release undo` writes it from {}",
                in_flight.revert_head()
            ),
        };
        bail!(
            "{} declares {}, which no tag names: a withdrawal is in flight, and {revert}; nothing moves until it lands",
            in_flight.branch,
            in_flight.version
        );
    }

    if let Some(group) = &args.group {
        let options = planner::CutOptions {
            major: false,
            group: Some(group.clone()),
            floor: args.floor,
        };
        let plan = planner::plan(Transition::Cut, options)?;
        let headline = format!("`{group}` bump: {}", name(Transition::Cut));
        println!("{headline}");
        github::add_summary(&summary(&headline))?;
        github::set_outputs_if_available(&Inferred {
            transition: name(Transition::Cut),
        })?;
        return carry_out(
            Transition::Cut,
            &plan,
            &groups,
            args.dry_run,
            args.into.as_deref(),
        );
    }

    let channel = args
        .channel
        .expect("clap requires one of --channel and --group");
    if channel == Channel::Nightly {
        if args.major {
            bail!("`--major` asks for a cut; a nightly bump has no transition");
        }
        let declared = groups.app_version()?;
        let (plan, report) = plan_nightly(&declared, args.dry_run)?;
        println!("{report}");
        github::add_summary(&summary(&report))?;
        if let Some(dir) = args.into.as_deref() {
            render_nightly(dir, plan.as_ref(), &report)?;
        }
        github::set_outputs_if_available(&Inferred {
            transition: "nightly".to_owned(),
        })?;
        return match plan {
            Some(plan) => github::set_outputs_if_available(&plan),
            None => Ok(()),
        };
    }

    let states = planner::series_states(&series, &tagged, &groups, root)?;
    let newest = states.iter().max_by_key(|state| state.declared.target());
    let transition = planner::infer(channel, newest, args.major)?;
    let headline = match newest {
        Some(state) => format!(
            "{channel} bump: {} — the newest series {} declares {}",
            name(transition),
            state.branch,
            state.declared
        ),
        None => format!("{channel} bump: {} — no series is open", name(transition)),
    };
    println!("{headline}");
    github::add_summary(&summary(&headline))?;
    github::set_outputs_if_available(&Inferred {
        transition: name(transition),
    })?;

    let plan = match transition {
        Transition::Cut => planner::plan(
            Transition::Cut,
            planner::CutOptions {
                major: args.major,
                group: None,
                floor: None,
            },
        )?,
        other => planner::plan_on_series(
            other,
            &newest.expect("a series transition has a series").declared,
            &groups,
            root,
        )?,
    };
    carry_out(
        transition,
        &plan,
        &groups,
        args.dry_run,
        args.into.as_deref(),
    )
}

/// Decides the day's nightly from `manifest`'s target: skip when nothing
/// landed, refuse a taken day, else mint it at `HEAD` — or, dry, report what
/// it would have. The plan is `None` where nothing is minted, or would be.
fn plan_nightly(manifest: &Version, dry_run: bool) -> Result<(Option<Plan>, String)> {
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

    match (dry_run, decision) {
        (true, Decision::New(version)) => {
            let plan = Plan::of(&version)?;
            let report = format!("Dry run: {verdict} Would mint {}.", plan.tag);
            Ok((Some(plan), report))
        }
        (true, _) => Ok((None, format!("Dry run: {verdict}"))),
        (false, Decision::New(version)) => {
            let head = git::run(&["rev-parse", "HEAD"])?;
            let plan = Plan::of(&version)?;
            github::create_tag(&plan.tag, &head)?;
            let report = format!("Tagged nightly {tag} at {head}.", tag = plan.tag);
            Ok((Some(plan), report))
        }
        (false, _) => Ok((None, verdict)),
    }
}

/// A nightly dry run's one rendering: the refs it would mint, or the verdict
/// that it would mint none, so the run leaves the artifact every rehearsal
/// leaves.
fn render_nightly(dir: &Path, plan: Option<&Plan>, report: &str) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let refs = match plan {
        Some(plan) => formatdoc! { "
            # Refs

            {report}

            - tag `{tag}` on `HEAD`, not minted
            ",
            tag = plan.tag,
        },
        None => formatdoc! { "
            # Refs

            {report}

            No tag would be minted.
            "
        },
    };
    let file = dir.join("refs.md");
    fs::write(&file, refs).with_context(|| format!("writing {}", file.display()))?;
    println!("  rendered {}", file.display());
    Ok(())
}

/// The job summary's heading and verdict; what a dry run rendered is the
/// run's artifact.
fn summary(headline: &str) -> String {
    formatdoc! { "
        ### Bump

        {headline}

        "
    }
}

/// Reports `plan`, then either renders it dry into `into` or writes it into
/// the tree and publishes its landing — a cut's `main` landing with its body
/// file, every other transition's series landing.
fn carry_out(
    transition: Transition,
    plan: &planner::Plan,
    groups: &release_groups::GroupFile,
    dry_run: bool,
    into: Option<&Path>,
) -> Result<()> {
    let landing = plan.landing(groups)?;

    match &landing {
        Some(landing) => {
            println!(
                "{transition:?}: {version} on {branch} ({channel}); tag {tag}",
                version = plan.version,
                branch = plan.branch,
                channel = plan.channel,
                tag = plan.tag,
            );
            println!("  {}", landing.describe(&plan.branch));
        }
        None => println!("{transition:?}: one library on main, no series"),
    }
    if let Some(main) = plan.main_landing(groups)? {
        if !plan.next_main_version.is_empty() {
            println!("  main moves to {}", plan.next_main_version);
        }
        println!("  {}", main.describe(MAIN));
        if main.title.starts_with("summary:") {
            for header in plan.summary_headers(groups)? {
                println!("    {header}");
            }
        }
    }
    for group in &plan.groups {
        println!("  {}", group.describe());
    }
    for pin in &plan.moves {
        println!("  {}", pin.describe());
    }

    if dry_run {
        match into {
            Some(dir) => {
                fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
                for (name, content) in planner::rehearse(transition, plan)? {
                    let file = dir.join(&name);
                    fs::write(&file, content)
                        .with_context(|| format!("writing {}", file.display()))?;
                    println!("  rendered {}", file.display());
                }
                println!("  (dry run — nothing written to the tree)");
            }
            None => println!("  (dry run — nothing written)"),
        }
        return github::set_outputs_if_available(plan);
    }

    planner::execute(transition, plan)?;
    github::set_outputs_if_available(plan)?;

    // A cut lands on `main` first; its series opens from that merge commit,
    // in a later step that publishes the series' landing itself.
    if transition == Transition::Cut {
        let main = plan.main_landing(groups)?.context("a cut lands on main")?;
        println!("  wrote the main-side manifests on main");
        let body = if main.title.starts_with("summary:") {
            formatdoc! { "
                {plan}

                {headers}
                ",
                plan = planner::pull_request_body(),
                headers = plan.summary_headers(groups)?.join("\n"),
            }
        } else {
            planner::pull_request_body()
        };
        let body_path = std::env::temp_dir().join("release-bump-body.md");
        fs::write(&body_path, body).with_context(|| format!("writing {}", body_path.display()))?;
        github::set_outputs_if_available(&main)?;
        return github::set_outputs_if_available(&BodyPath {
            body_path: body_path.to_string_lossy().into_owned(),
        });
    }

    let landing = landing.context("a transition on a series has a landing")?;
    println!("  wrote {} on {}", plan.version, plan.branch);
    github::set_outputs_if_available(&landing)
}

/// Where the main pull request's body was written: a summary's headers span
/// lines, which a step output cannot carry.
#[derive(serde::Serialize)]
struct BodyPath {
    body_path: String,
}

/// The transition as the command line spells it.
fn name(transition: Transition) -> String {
    transition
        .to_possible_value()
        .expect("every transition has a name")
        .get_name()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, clap::Error> {
        Options::try_parse_from(std::iter::once("bump").chain(args.iter().copied()))
    }

    /// `--major` belongs to a channel's cut and `--floor` to a group's; each
    /// is refused with the other target, as are both targets, and neither.
    #[test]
    fn a_flag_is_refused_with_the_target_it_does_not_belong_to() {
        assert!(parse(&["--channel", "preview", "--major"]).is_ok());
        assert!(parse(&["--group", "bedrock", "--floor", "minor"]).is_ok());
        for refused in [
            &["--group", "bedrock", "--major"][..],
            &["--channel", "preview", "--floor", "minor"],
            &["--major"],
            &["--group", "bedrock", "--channel", "preview"],
        ] {
            assert!(parse(refused).is_err(), "{refused:?} parsed");
        }
    }

    /// A nightly rehearsal renders the refs it would mint, or that it would
    /// mint none, so the run's artifact exists either way.
    #[test]
    fn a_nightly_rehearsal_renders_its_refs() {
        let dir = tempfile::tempdir().unwrap();
        let plan = Plan::of(&Version::parse("0.2.0-nightly.20261125").unwrap()).unwrap();
        render_nightly(dir.path(), Some(&plan), "Dry run: work landed.").unwrap();
        let refs = fs::read_to_string(dir.path().join("refs.md")).unwrap();
        assert!(refs.contains("`v0.2.0-nightly.20261125`"), "{refs}");
        assert!(refs.contains("not minted"), "{refs}");

        render_nightly(dir.path(), None, "Dry run: nothing landed.").unwrap();
        let refs = fs::read_to_string(dir.path().join("refs.md")).unwrap();
        assert!(refs.contains("No tag would be minted."), "{refs}");
    }
}
