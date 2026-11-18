// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plans a release-cycle transition and leaves its version write in the tree
//! for the pull-request step — or, on a dry run, renders what it would have
//! written. See [`xtask::planners::release`].

use anyhow::{Context as _, Result};
use clap::Parser;
use std::{fs, path::PathBuf};
use xtask::{
    planners::release as planner, release_groups, releases::version::Transition, tools::github,
    workspace,
};

#[derive(Parser)]
pub struct Options {
    #[arg(value_enum)]
    transition: Transition,
    /// Cut toward the app's next major. A `Covenant: break` since the last
    /// stable refuses a cut without this; with it, the major is cut whether
    /// or not a break was declared.
    #[arg(long, conflicts_with = "group")]
    major: bool,
    /// Cut one library group alone: mint its tag on `main`, open no series,
    /// and move nothing else. Refused when the group has no counted commits.
    #[arg(long)]
    group: Option<String>,
    /// Raise a group-scoped cut's bump to at least this; never lowers it.
    #[arg(long, value_enum, requires = "group")]
    floor: Option<planner::Floor>,
    /// Plan and print without creating a ref or writing a file.
    #[arg(long)]
    dry_run: bool,
    /// The directory a dry run renders into: the version writes as patches,
    /// the refs and pull requests it would open, and the notes for the tag.
    #[arg(long, requires = "dry_run")]
    into: Option<PathBuf>,
}

pub fn run(args: Options) -> Result<()> {
    if args.transition != Transition::Cut && (args.major || args.group.is_some()) {
        anyhow::bail!(
            "`--major` and `--group` apply to a cut; `{:?}` moves a series mechanically",
            args.transition
        );
    }
    let options = planner::CutOptions {
        major: args.major,
        group: args.group.clone(),
        floor: args.floor,
    };
    let plan = planner::plan(args.transition, options)?;
    let groups = release_groups::load(&workspace::load()?)?;
    let landing = plan.landing(&groups)?;

    match &landing {
        Some(landing) => {
            println!(
                "{transition:?}: {version} on {branch} ({channel}); tag {tag}",
                transition = args.transition,
                version = plan.version,
                branch = plan.branch,
                channel = plan.channel,
                tag = plan.tag,
            );
            println!("  {}", landing.describe(&plan.branch));
        }
        None => println!(
            "{transition:?}: one library on main, no series",
            transition = args.transition
        ),
    }
    if let Some(main) = plan.main_landing(&groups)? {
        if !plan.next_main_version.is_empty() {
            println!("  main moves to {}", plan.next_main_version);
        }
        println!("  {}", main.describe("main"));
        if main.title.starts_with("summary:") {
            for header in plan.summary_headers(&groups)? {
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

    if args.dry_run {
        match &args.into {
            Some(dir) => {
                fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
                for (name, content) in planner::rehearse(args.transition, &plan)? {
                    let file = dir.join(&name);
                    fs::write(&file, content)
                        .with_context(|| format!("writing {}", file.display()))?;
                    println!("  rendered {}", file.display());
                }
                println!("  (dry run — nothing written to the tree)");
            }
            None => println!("  (dry run — nothing written)"),
        }
        return github::set_outputs_if_available(&plan);
    }

    planner::execute(args.transition, &plan)?;
    github::set_outputs_if_available(&plan)?;

    // A cut lands on `main` first; its series opens from that merge commit,
    // in a later step that publishes the series' landing itself.
    if args.transition == Transition::Cut {
        let main = plan.main_landing(&groups)?.context("a cut lands on main")?;
        println!("  wrote the main-side manifests on main");
        let mut body = planner::pull_request_body();
        if main.title.starts_with("summary:") {
            body.push_str("\n\n");
            body.push_str(&plan.summary_headers(&groups)?.join("\n"));
            body.push('\n');
        }
        let body_path = std::env::temp_dir().join("release-plan-body.md");
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The workflow dispatches `release-plan <transition> [--dry-run [--into
    /// <dir>]]`; the transition is a positional value, and the flags follow it.
    #[test]
    fn the_cli_takes_a_transition_and_a_dry_run_flag() {
        let args = Options::try_parse_from(["release-plan", "cut", "--dry-run"]).unwrap();
        assert_eq!(args.transition, Transition::Cut);
        assert!(args.dry_run && args.into.is_none());

        let args =
            Options::try_parse_from(["release-plan", "cut", "--dry-run", "--into", "out"]).unwrap();
        assert_eq!(args.into.as_deref(), Some(std::path::Path::new("out")));

        let args = Options::try_parse_from(["release-plan", "patch"]).unwrap();
        assert_eq!(args.transition, Transition::Patch);
        assert!(!args.dry_run && !args.major);

        // A major is asked for by flag, never computed.
        let args =
            Options::try_parse_from(["release-plan", "cut", "--major", "--dry-run"]).unwrap();
        assert!(args.major && args.dry_run);

        // A library alone, with the least it must bump.
        let args = Options::try_parse_from([
            "release-plan",
            "cut",
            "--group",
            "bedrock",
            "--floor",
            "minor",
            "--dry-run",
        ])
        .unwrap();
        assert_eq!(args.group.as_deref(), Some("bedrock"));
        assert_eq!(args.floor, Some(planner::Floor::Minor));

        // A floor needs a group, and a major is the app's alone.
        assert!(Options::try_parse_from(["release-plan", "cut", "--floor", "minor"]).is_err());
        assert!(
            Options::try_parse_from(["release-plan", "cut", "--group", "bedrock", "--major"])
                .is_err()
        );

        // A rendering directory is a dry run's; a real run writes the tree.
        assert!(Options::try_parse_from(["release-plan", "cut", "--into", "out"]).is_err());
        assert!(Options::try_parse_from(["release-plan", "ship"]).is_err());
    }
}
