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

use super::{plan::carry_out, prepare::plan_nightly};
use anyhow::{Result, bail};
use clap::{ArgGroup, Parser, ValueEnum as _};
use serde::Serialize;
use std::path::PathBuf;
use xtask::{
    Channel,
    constants::MAIN,
    planners::release as planner,
    release_groups,
    releases::version::Transition,
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
        let (plan, report) = plan_nightly(&declared, None, args.dry_run, true)?;
        println!("{report}");
        github::set_outputs_if_available(&Inferred {
            transition: "nightly".to_owned(),
        })?;
        return github::set_outputs_if_available(&plan);
    }

    let states = planner::series_states(&series, &tagged, &groups, root)?;
    let newest = states.iter().max_by_key(|state| state.declared.target());
    let transition = planner::infer(channel, newest, args.major)?;
    match newest {
        Some(state) => println!(
            "{channel} bump: {} — the newest series {} declares {}",
            name(transition),
            state.branch,
            state.declared
        ),
        None => println!("{channel} bump: {} — no series is open", name(transition)),
    }
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
}
