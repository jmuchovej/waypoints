// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `dev` — the workstation's tools.

use anyhow::Result;
use clap::Subcommand;

pub mod devenv_files;
#[cfg(feature = "local-dev")]
pub mod server;

#[derive(Subcommand)]
pub enum Command {
    /// Serves the Waypoints web UI's HTTP backend for browser-based dev.
    #[cfg(feature = "local-dev")]
    Server(server::Options),
    /// Prints the files devenv materializes into the working tree.
    DevenvFiles(devenv_files::Options),
}

pub fn run(command: Command) -> Result<()> {
    match command {
        #[cfg(feature = "local-dev")]
        Command::Server(args) => server::run(args),
        Command::DevenvFiles(args) => devenv_files::run(args),
    }
}
