// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of Waypoints.
//
// Waypoints is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Waypoints is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.
//
// Additional permission under GNU AGPL version 3 section 7: this Program
// may be conveyed through application stores on the terms set out in
// docs/licensing/COPYING.exceptions.

//! `cargo xtask` — the CLI over the `xtask` library.
//!
//! Each task under `tasks/` is a clap argument struct and a `run`: it parses,
//! orchestrates the library, and prints. Logic worth testing lives in the
//! library, where every task can share it.
//!
//! The commands are grouped by verb: `release` is the pipeline, `check` the
//! pull-request checks, `generate` what `check-generated` holds to its
//! source, `dev` the workstation's tools, and `test-plan` the one query the
//! hooks and the test jobs read.

#![allow(clippy::disallowed_macros)]

use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod tasks {
    #[cfg(feature = "local-dev")]
    pub mod dev_server;
    pub mod devenv_files;
    #[cfg(feature = "local-dev")]
    pub mod icongen;
    pub mod license;
    pub mod publish_plan;
    pub mod release_groups;
    pub mod release_index;
    pub mod release_land;
    pub mod release_manifest;
    pub mod release_plan;
    pub mod release_stage;
    pub mod release_upload;
    pub mod release_withdraw;
    pub mod semver_check;
    pub mod test_plan;
    pub mod typegen;
    pub mod version_integrity;
    pub mod workflows;
}

#[derive(Parser)]
// Without `local-dev` the binary is the one `cargo ci-task` runs, so it says so;
// `bin_name` because clap otherwise prints argv[0] in the usage line.
#[cfg_attr(
    feature = "local-dev",
    command(name = "cargo xtask", bin_name = "cargo xtask")
)]
#[cfg_attr(
    not(feature = "local-dev"),
    command(name = "cargo ci-task", bin_name = "cargo ci-task")
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Prints the nextest filterset a CI run should use.
    TestPlan(tasks::test_plan::Options),
    /// The release pipeline: planning, staging, publishing, and landing.
    #[command(subcommand)]
    Release(Release),
    /// The checks a pull request must pass.
    #[command(subcommand)]
    Check(Check),
    /// What is generated into the tree, and held to its source.
    #[command(subcommand)]
    Generate(Generate),
    /// The workstation's tools.
    #[command(subcommand)]
    Dev(Dev),
}

#[derive(Subcommand)]
enum Release {
    /// Plans a channel's run: what it builds, whether it publishes, and the tag nightly mints.
    Prepare(tasks::publish_plan::Options),
    /// Plans a release-cycle transition: cut, advance, promote, or patch.
    Plan(tasks::release_plan::Options),
    /// Copies one platform's published artifacts into `release/` under their store keys.
    Stage(tasks::release_stage::Options),
    /// Uploads published artifacts to the releases bucket.
    Upload(tasks::release_upload::Options),
    /// Writes a channel's manifest after every required target has uploaded.
    Manifest(tasks::release_manifest::Options),
    /// Best-effort deletes the objects a shortfall run uploaded.
    Withdraw(tasks::release_withdraw::Options),
    /// Writes a published tag's release-index entry for its pull request to land.
    Index(tasks::release_index::Options),
    /// Takes a bot's pull request from opened to landed, one step at a time.
    Land(tasks::release_land::Options),
    /// The group file: its generated schema, and its projection into the manifests.
    Groups(tasks::release_groups::Options),
}

#[derive(Subcommand)]
enum Check {
    /// Holds the tree to the licensing policy.
    Licenses,
    /// Measures each touched library group's stable surface against its last tag.
    Semver(tasks::semver_check::Options),
    /// Checks what a pull request claims about versions.
    VersionIntegrity(tasks::version_integrity::Options),
}

#[derive(Subcommand)]
enum Generate {
    /// Regenerates every checked-in GitHub Actions workflow, and the checks OpenTofu requires of them.
    Workflows(tasks::workflows::Options),
    /// Regenerates the TypeScript bindings from the Rust data model.
    Bindings(tasks::typegen::Options),
    /// Regenerates every app icon from the authored artwork in `src-tauri/res/`.
    #[cfg(feature = "local-dev")]
    Icons(tasks::icongen::Options),
    /// Regenerates the third-party notices from the registry and the lockfiles.
    Notices,
}

#[derive(Subcommand)]
enum Dev {
    /// Serves the Waypoints web UI's HTTP backend for browser-based dev.
    #[cfg(feature = "local-dev")]
    Server(tasks::dev_server::Options),
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(tasks::devenv_files::Options),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::TestPlan(args) => tasks::test_plan::run(args),
        Command::Release(release) => match release {
            Release::Prepare(args) => tasks::publish_plan::run(args),
            Release::Plan(args) => tasks::release_plan::run(args),
            Release::Stage(args) => tasks::release_stage::run(args),
            Release::Upload(args) => tasks::release_upload::run(args),
            Release::Manifest(args) => tasks::release_manifest::run(args),
            Release::Withdraw(args) => tasks::release_withdraw::run(args),
            Release::Index(args) => tasks::release_index::run(args),
            Release::Land(args) => tasks::release_land::run(args),
            Release::Groups(args) => tasks::release_groups::run(args),
        },
        Command::Check(check) => match check {
            Check::Licenses => tasks::license::check(),
            Check::Semver(args) => tasks::semver_check::run(args),
            Check::VersionIntegrity(args) => tasks::version_integrity::run(args),
        },
        Command::Generate(generate) => match generate {
            Generate::Workflows(args) => tasks::workflows::run(args),
            Generate::Bindings(args) => tasks::typegen::run(args),
            #[cfg(feature = "local-dev")]
            Generate::Icons(args) => tasks::icongen::run(args),
            Generate::Notices => tasks::license::notices(),
        },
        Command::Dev(dev) => match dev {
            #[cfg(feature = "local-dev")]
            Dev::Server(args) => tasks::dev_server::run(args),
            Dev::DevenvFiles(args) => tasks::devenv_files::run(args),
        },
    }
}
