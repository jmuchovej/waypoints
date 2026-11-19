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
//! The tree under `tasks/` is the command line: `tasks/<verb>.rs` declares a
//! verb's subcommands and dispatches them, and `tasks/<verb>/<task>.rs` is one
//! task — a clap argument struct and a `run` that parses, orchestrates the
//! library, and prints. Logic worth testing lives in the library, where every
//! task can share it.
//!
//! `release` is the pipeline, `check` the pull-request checks, `generate` what
//! `check-generated` holds to its source, `dev` the workstation's tools, and
//! `test-plan` the one query the hooks and the test jobs read.

#![allow(clippy::disallowed_macros)]

use anyhow::Result;
use clap::{Parser, Subcommand};

mod tasks {
    pub mod check;
    pub mod dev;
    pub mod generate;
    pub mod release;
    pub mod test_plan;
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
    Release(tasks::release::Command),
    /// The checks a pull request must pass.
    #[command(subcommand)]
    Check(tasks::check::Command),
    /// What is generated into the tree, and held to its source.
    #[command(subcommand)]
    Generate(tasks::generate::Command),
    /// The workstation's tools.
    #[command(subcommand)]
    Dev(tasks::dev::Command),
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::TestPlan(args) => tasks::test_plan::run(args),
        Command::Release(command) => tasks::release::run(command),
        Command::Check(command) => tasks::check::run(command),
        Command::Generate(command) => tasks::generate::run(command),
        Command::Dev(command) => tasks::dev::run(command),
    }
}
