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
    /// Copies one platform's published artifacts into `release/` under their store keys.
    ReleaseStage(tasks::ReleaseStageArgs),
    /// Uploads published artifacts to the releases bucket.
    ReleaseUpload(tasks::ReleaseUploadArgs),
    /// Writes a channel's manifest after every required target has uploaded.
    ReleaseManifest(tasks::ReleaseManifestArgs),
    /// Best-effort deletes the objects a shortfall run uploaded.
    ReleaseWithdraw(tasks::ReleaseWithdrawArgs),
    /// Plans a release-cycle transition: cut, advance, promote, or patch.
    ReleasePlan(tasks::ReleasePlanArgs),
    /// Records a published tag in the release index on `main`.
    ReleaseIndex(tasks::ReleaseIndexArgs),
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(tasks::DevenvFilesArgs),
    /// Regenerates every checked-in GitHub Actions workflow.
    #[command(alias = "gen-workflows")]
    Workflows(tasks::WorkflowArgs),
    /// Regenerates the TypeScript bindings from the Rust data model.
    Typegen(tasks::TypegenArgs),
    /// Serves the Waypoints web UI's HTTP backend for browser-based dev.
    DevServer(tasks::DevServerArgs),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::TestPlan(args) => tasks::test_plan::run(args),
        Command::NightlyPlan(args) => tasks::nightly_plan::run(args),
        Command::ReleaseStage(args) => tasks::release_stage::run(args),
        Command::ReleaseUpload(args) => tasks::release_upload::run(args),
        Command::ReleaseManifest(args) => tasks::release_manifest::run(args),
        Command::ReleaseWithdraw(args) => tasks::release_withdraw::run(args),
        Command::ReleasePlan(args) => tasks::release_plan::run(args),
        Command::ReleaseIndex(args) => tasks::release_index::run(args),
        Command::DevenvFiles(args) => tasks::devenv_files::run(args),
        Command::Workflows(args) => tasks::workflows::run(args),
        Command::Typegen(args) => tasks::typegen::run(args),
        Command::DevServer(args) => tasks::dev_server::run(args),
    }
}
