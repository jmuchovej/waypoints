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
//! The tree under `tasks/` is the command line: one file per task, grouped by
//! verb, each a clap argument struct and a `run` that parses, orchestrates
//! the library, and prints. Logic worth testing lives in the library, where
//! every task can share it.
//!
//! `release` is the pipeline, `check` the pull-request checks, `generate` what
//! `check-generated` holds to its source, `dev` the workstation's tools, and
//! `test-plan` the one query the hooks and the test jobs read.

#![allow(clippy::disallowed_macros)]

use anyhow::Result;
use clap::{Parser, Subcommand};

mod tasks {
    pub mod test_plan;

    /// The pipeline: planning, staging, publishing, and landing.
    pub mod release {
        pub mod groups;
        pub mod index;
        pub mod land;
        pub mod manifest;
        pub mod plan;
        pub mod prepare;
        pub mod stage;
        pub mod upload;
        pub mod withdraw;
    }

    /// The checks a pull request must pass.
    pub mod check {
        pub mod licenses;
        pub mod semver;
        pub mod version_integrity;
    }

    /// What is generated into the tree, and held to its source.
    pub mod generate {
        pub mod bindings;
        #[cfg(feature = "local-dev")]
        pub mod icons;
        pub mod notices;
        pub mod workflows;
    }

    /// The workstation's tools.
    pub mod dev {
        pub mod devenv_files;
        #[cfg(feature = "local-dev")]
        pub mod server;
    }
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
    Prepare(tasks::release::prepare::Options),
    /// Plans a release-cycle transition: cut, advance, promote, or patch.
    Plan(tasks::release::plan::Options),
    /// Copies one platform's published artifacts into `release/` under their store keys.
    Stage(tasks::release::stage::Options),
    /// Uploads published artifacts to the releases bucket.
    Upload(tasks::release::upload::Options),
    /// Writes a channel's manifest after every required target has uploaded.
    Manifest(tasks::release::manifest::Options),
    /// Best-effort deletes the objects a shortfall run uploaded.
    Withdraw(tasks::release::withdraw::Options),
    /// Writes a published tag's release-index entry for its pull request to land.
    Index(tasks::release::index::Options),
    /// Takes a bot's pull request from opened to landed, one step at a time.
    Land(tasks::release::land::Options),
    /// The group file: its generated schema, and its projection into the manifests.
    Groups(tasks::release::groups::Options),
}

#[derive(Subcommand)]
enum Check {
    /// Holds the tree to the licensing policy.
    Licenses(tasks::check::licenses::Options),
    /// Measures each touched library group's stable surface against its last tag.
    Semver(tasks::check::semver::Options),
    /// Checks what a pull request claims about versions.
    VersionIntegrity(tasks::check::version_integrity::Options),
}

#[derive(Subcommand)]
enum Generate {
    /// Regenerates every checked-in GitHub Actions workflow, and the checks OpenTofu requires of them.
    Workflows(tasks::generate::workflows::Options),
    /// Regenerates the TypeScript bindings from the Rust data model.
    Bindings(tasks::generate::bindings::Options),
    /// Regenerates every app icon from the authored artwork in `src-tauri/res/`.
    #[cfg(feature = "local-dev")]
    Icons(tasks::generate::icons::Options),
    /// Regenerates the third-party notices from the registry and the lockfiles.
    Notices(tasks::generate::notices::Options),
}

#[derive(Subcommand)]
enum Dev {
    /// Serves the Waypoints web UI's HTTP backend for browser-based dev.
    #[cfg(feature = "local-dev")]
    Server(tasks::dev::server::Options),
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(tasks::dev::devenv_files::Options),
}

fn main() -> Result<()> {
    use tasks::{check, dev, generate, release};

    match Args::parse().command {
        Command::TestPlan(args) => tasks::test_plan::run(args),
        Command::Release(command) => match command {
            Release::Prepare(args) => release::prepare::run(args),
            Release::Plan(args) => release::plan::run(args),
            Release::Stage(args) => release::stage::run(args),
            Release::Upload(args) => release::upload::run(args),
            Release::Manifest(args) => release::manifest::run(args),
            Release::Withdraw(args) => release::withdraw::run(args),
            Release::Index(args) => release::index::run(args),
            Release::Land(args) => release::land::run(args),
            Release::Groups(args) => release::groups::run(args),
        },
        Command::Check(command) => match command {
            Check::Licenses(args) => check::licenses::run(args),
            Check::Semver(args) => check::semver::run(args),
            Check::VersionIntegrity(args) => check::version_integrity::run(args),
        },
        Command::Generate(command) => match command {
            Generate::Workflows(args) => generate::workflows::run(args),
            Generate::Bindings(args) => generate::bindings::run(args),
            #[cfg(feature = "local-dev")]
            Generate::Icons(args) => generate::icons::run(args),
            Generate::Notices(args) => generate::notices::run(args),
        },
        Command::Dev(command) => match command {
            #[cfg(feature = "local-dev")]
            Dev::Server(args) => dev::server::run(args),
            Dev::DevenvFiles(args) => dev::devenv_files::run(args),
        },
    }
}
