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
    /// Plan and print without creating a ref or writing a file.
    #[arg(long)]
    dry_run: bool,
    /// The directory a dry run renders into: the version writes as patches,
    /// the refs and pull requests it would open, and the notes for the tag.
    #[arg(long, requires = "dry_run")]
    into: Option<PathBuf>,
}

pub fn run(args: Options) -> Result<()> {
    let plan = planner::plan(args.transition)?;
    let groups = release_groups::load(&workspace::load()?)?;
    let landing = plan.landing(&groups)?;

    println!(
        "{transition:?}: {version} on {branch} ({channel}); tag {tag}",
        transition = args.transition,
        version = plan.version,
        branch = plan.branch,
        channel = plan.channel,
        tag = plan.tag,
    );
    println!("  {}", landing.describe(&plan.branch));
    if let Some(main) = plan.main_landing(&groups)? {
        println!("  main moves to {}", plan.next_main_version);
        println!("  {}", main.describe("main"));
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
    println!("  wrote {} on {}", plan.version, plan.branch);

    github::set_outputs_if_available(&plan)?;
    github::set_outputs_if_available(&landing)
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
        assert!(!args.dry_run);

        // A rendering directory is a dry run's; a real run writes the tree.
        assert!(Options::try_parse_from(["release-plan", "cut", "--into", "out"]).is_err());
        assert!(Options::try_parse_from(["release-plan", "ship"]).is_err());
    }
}
