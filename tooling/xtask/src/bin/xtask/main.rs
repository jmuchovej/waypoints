//! `cargo xtask` — the CLI over the `xtask` library.
//!
//! Each task under `tasks/` is a clap argument struct and a `run`: it parses,
//! orchestrates the library, and prints. Logic worth testing lives in the
//! library, where every task can share it.

#![allow(clippy::disallowed_macros)]

use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod tasks {
    pub mod dev_server;
    pub mod devenv_files;
    pub mod icongen;
    pub mod nightly_plan;
    pub mod release_index;
    pub mod release_manifest;
    pub mod release_plan;
    pub mod release_stage;
    pub mod release_upload;
    pub mod release_withdraw;
    pub mod test_plan;
    pub mod typegen;
    pub mod workflows;
}

#[derive(Parser)]
#[command(name = "cargo xtask")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Prints the nextest filterset a CI run should use.
    TestPlan(tasks::test_plan::Options),
    /// Mints the release tag for a Nightly workflow run.
    NightlyPlan(tasks::nightly_plan::Options),
    /// Copies one platform's published artifacts into `release/` under their store keys.
    ReleaseStage(tasks::release_stage::Options),
    /// Uploads published artifacts to the releases bucket.
    ReleaseUpload(tasks::release_upload::Options),
    /// Writes a channel's manifest after every required target has uploaded.
    ReleaseManifest(tasks::release_manifest::Options),
    /// Best-effort deletes the objects a shortfall run uploaded.
    ReleaseWithdraw(tasks::release_withdraw::Options),
    /// Plans a release-cycle transition: cut, advance, promote, or patch.
    ReleasePlan(tasks::release_plan::Options),
    /// Records a published tag in the release index on `main`.
    ReleaseIndex(tasks::release_index::Options),
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(tasks::devenv_files::Options),
    /// Regenerates every checked-in GitHub Actions workflow.
    #[command(alias = "gen-workflows")]
    Workflows(tasks::workflows::Options),
    /// Regenerates the TypeScript bindings from the Rust data model.
    Typegen(tasks::typegen::Options),
    /// Regenerates every app icon from the authored artwork in `src-tauri/res/`.
    Icongen(tasks::icongen::Options),
    /// Serves the Waypoints web UI's HTTP backend for browser-based dev.
    DevServer(tasks::dev_server::Options),
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
        Command::Icongen(args) => tasks::icongen::run(args),
        Command::DevServer(args) => tasks::dev_server::run(args),
    }
}
