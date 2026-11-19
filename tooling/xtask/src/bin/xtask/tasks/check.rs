// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `check` — the checks a pull request must pass.

use anyhow::Result;
use clap::Subcommand;

pub mod licenses;
pub mod semver;
pub mod version_integrity;

#[derive(Subcommand)]
pub enum Command {
    /// Holds the tree to the licensing policy.
    Licenses,
    /// Measures each touched library group's stable surface against its last tag.
    Semver(semver::Options),
    /// Checks what a pull request claims about versions.
    VersionIntegrity(version_integrity::Options),
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Licenses => licenses::run(),
        Command::Semver(args) => semver::run(args),
        Command::VersionIntegrity(args) => version_integrity::run(args),
    }
}
