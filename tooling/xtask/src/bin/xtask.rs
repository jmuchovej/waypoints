use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use xtask::tasks;

#[derive(Parser)]
#[command(name = "cargo xtask")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Prints the nextest filterset a CI run should use.
    TestPlan(tasks::TestPlanArgs),
    /// Mints the release tag for a Nightly workflow run.
    NightlyPlan(tasks::NightlyPlanArgs),
    /// Plans a release-cycle transition: cut, advance, promote, or patch.
    ReleasePlan(tasks::ReleasePlanArgs),
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(tasks::DevenvFilesArgs),
    /// Regenerates every checked-in GitHub Actions workflow.
    #[command(alias = "gen-workflows")]
    Workflows(tasks::WorkflowArgs),
    /// Serves the Waypoints web UI's HTTP backend for browser-based dev.
    DevServer(tasks::DevServerArgs),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::TestPlan(args) => tasks::test_plan::run(args),
        Command::NightlyPlan(args) => tasks::nightly_plan::run(args),
        Command::ReleasePlan(args) => tasks::release_plan::run(args),
        Command::DevenvFiles(args) => tasks::devenv_files::run(args),
        Command::Workflows(args) => tasks::workflows::run(args),
        Command::DevServer(args) => tasks::dev_server::run(args),
    }
}
