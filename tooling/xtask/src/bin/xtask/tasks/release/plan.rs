// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plans a release-cycle transition and leaves its version write in the tree
//! for the pull-request step — or, on a dry run, renders what it would have
//! written. See [`xtask::planners::release`].

use anyhow::{Context as _, Result};
use clap::Parser;
use indoc::formatdoc;
use std::{
    fs,
    path::{Path, PathBuf},
};
use xtask::{
    constants::MAIN, planners::release as planner, release_groups, releases::version::Transition,
    tools::github, workspace,
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
    carry_out(
        args.transition,
        &plan,
        &groups,
        args.dry_run,
        args.into.as_deref(),
    )
}

/// Reports `plan`, then either renders it dry into `into` or writes it into
/// the tree and publishes its landing — a cut's `main` landing with its body
/// file, every other transition's series landing. Shared with `release
/// bump`, which plans the same way once it has inferred the transition.
pub(crate) fn carry_out(
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
