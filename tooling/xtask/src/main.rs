pub mod tasks {
    pub mod nightly_plan;
    pub mod workflows;
}
pub mod utils {
    pub mod devenv;
}

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo xtask")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Resolves the release tag for a Nightly workflow run.
    NightlyPlan(tasks::nightly_plan::NightlyPlanArgs),
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(utils::devenv::DevenvFilesArgs),
    /// Regenerates every checked-in GitHub Actions workflow.
    Workflows(tasks::workflows::WorkflowArgs),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::NightlyPlan(args) => tasks::nightly_plan::run(args),
        Command::DevenvFiles(args) => utils::devenv::run(args),
        Command::Workflows(args) => tasks::workflows::run(args),
    }
}
