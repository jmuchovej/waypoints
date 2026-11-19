// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `generate` — what is generated into the tree, and held to its source.

use anyhow::Result;
use clap::Subcommand;

pub mod bindings;
#[cfg(feature = "local-dev")]
pub mod icons;
pub mod notices;
pub mod workflows;

#[derive(Subcommand)]
pub enum Command {
    /// Regenerates every checked-in GitHub Actions workflow, and the checks OpenTofu requires of them.
    Workflows(workflows::Options),
    /// Regenerates the TypeScript bindings from the Rust data model.
    Bindings(bindings::Options),
    /// Regenerates every app icon from the authored artwork in `src-tauri/res/`.
    #[cfg(feature = "local-dev")]
    Icons(icons::Options),
    /// Regenerates the third-party notices from the registry and the lockfiles.
    Notices,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Workflows(args) => workflows::run(args),
        Command::Bindings(args) => bindings::run(args),
        #[cfg(feature = "local-dev")]
        Command::Icons(args) => icons::run(args),
        Command::Notices => notices::run(),
    }
}
