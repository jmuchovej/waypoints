//! Plans a release-cycle transition and mints its refs. See
//! [`xtask::planners::release`].

use anyhow::Result;
use clap::Parser;
use xtask::{planners::release as planner, releases::version::Transition, tools::github};

#[derive(Parser)]
pub struct Options {
    #[arg(value_enum)]
    transition: Transition,
    /// Plan and print without creating any ref.
    #[arg(long)]
    dry_run: bool,
}

pub fn run(args: Options) -> Result<()> {
    let plan = planner::plan(args.transition)?;

    println!(
        "{transition:?}: {version} on {branch} ({channel}); tag {tag}",
        transition = args.transition,
        version = plan.version,
        branch = plan.branch,
        channel = plan.channel,
        tag = plan.tag,
    );
    if !plan.next_main_version.is_empty() {
        println!("  main moves to {}", plan.next_main_version);
    }

    if args.dry_run {
        println!("  (dry run — nothing written)");
        return github::set_outputs_if_available(&plan);
    }

    let executed = planner::execute(args.transition, &plan)?;
    println!(
        "  tagged {tag} at {commit}",
        tag = plan.tag,
        commit = executed.commit
    );
    if let Some((next, bumped)) = executed.main_bump {
        println!("  main bumped to {next} at {bumped}");
    }

    github::set_outputs_if_available(&plan)
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
