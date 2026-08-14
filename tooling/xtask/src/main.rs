#![warn(unused, dead_code)]

pub mod tasks {
    pub mod devenv_files;
    pub mod nightly_plan;
    pub mod test_plan;
    pub mod workflows;

    pub use devenv_files::DevenvFilesArgs;
    pub use nightly_plan::NightlyPlanArgs;
    pub use test_plan::TestPlanArgs;
    pub use workflows::WorkflowArgs;
}
pub mod utils {
    pub mod devenv;
    pub mod github;
}
pub mod shell;
pub mod workspace;

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
    /// Prints the nextest filterset a CI run should use.
    TestPlan(tasks::TestPlanArgs),
    /// Resolves the release tag for a Nightly workflow run.
    NightlyPlan(tasks::NightlyPlanArgs),
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(tasks::DevenvFilesArgs),
    /// Regenerates every checked-in GitHub Actions workflow.
    #[command(alias = "gen-workflows")]
    Workflows(tasks::WorkflowArgs),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::TestPlan(args) => tasks::test_plan::run(args),
        Command::NightlyPlan(args) => tasks::nightly_plan::run(args),
        Command::DevenvFiles(args) => tasks::devenv_files::run(args),
        Command::Workflows(args) => tasks::workflows::run(args),
    }
}
