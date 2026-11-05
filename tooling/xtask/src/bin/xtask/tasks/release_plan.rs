// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plans a release-cycle transition and leaves its version write in the tree
//! for the pull-request step. See [`xtask::planners::release`].

use anyhow::Result;
use clap::Parser;
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
        println!("  (dry run — nothing written)");
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

    /// The workflow dispatches `release-plan <transition> [--dry-run]`; the
    /// transition is a positional value, and the flag follows it.
    #[test]
    fn the_cli_takes_a_transition_and_a_dry_run_flag() {
        let args = Options::try_parse_from(["release-plan", "cut", "--dry-run"]).unwrap();
        assert_eq!(args.transition, Transition::Cut);
        assert!(args.dry_run);

        let args = Options::try_parse_from(["release-plan", "patch"]).unwrap();
        assert_eq!(args.transition, Transition::Patch);
        assert!(!args.dry_run);

        assert!(Options::try_parse_from(["release-plan", "ship"]).is_err());
    }
}
