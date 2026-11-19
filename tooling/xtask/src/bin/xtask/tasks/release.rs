// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `release` — the pipeline: planning, staging, publishing, and landing.

use anyhow::Result;
use clap::Subcommand;

pub mod groups;
pub mod index;
pub mod land;
pub mod manifest;
pub mod plan;
pub mod prepare;
pub mod stage;
pub mod upload;
pub mod withdraw;

#[derive(Subcommand)]
pub enum Command {
    /// Plans a channel's run: what it builds, whether it publishes, and the tag nightly mints.
    Prepare(prepare::Options),
    /// Plans a release-cycle transition: cut, advance, promote, or patch.
    Plan(plan::Options),
    /// Copies one platform's published artifacts into `release/` under their store keys.
    Stage(stage::Options),
    /// Uploads published artifacts to the releases bucket.
    Upload(upload::Options),
    /// Writes a channel's manifest after every required target has uploaded.
    Manifest(manifest::Options),
    /// Best-effort deletes the objects a shortfall run uploaded.
    Withdraw(withdraw::Options),
    /// Writes a published tag's release-index entry for its pull request to land.
    Index(index::Options),
    /// Takes a bot's pull request from opened to landed, one step at a time.
    Land(land::Options),
    /// The group file: its generated schema, and its projection into the manifests.
    Groups(groups::Options),
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Prepare(args) => prepare::run(args),
        Command::Plan(args) => plan::run(args),
        Command::Stage(args) => stage::run(args),
        Command::Upload(args) => upload::run(args),
        Command::Manifest(args) => manifest::run(args),
        Command::Withdraw(args) => withdraw::run(args),
        Command::Index(args) => index::run(args),
        Command::Land(args) => land::run(args),
        Command::Groups(args) => groups::run(args),
    }
}
