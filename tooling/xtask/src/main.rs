pub(crate) mod tasks {
    pub(crate) mod workflows;
}
pub(crate) mod utils {
    pub(crate) mod devenv;
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
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(utils::devenv::DevenvFilesArgs),
    /// Regenerates every checked-in GitHub Actions workflow.
    Workflows(tasks::workflows::WorkflowArgs),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::DevenvFiles(args) => utils::devenv::run(args),
        Command::Workflows(args) => tasks::workflows::run(args),
    }
}
