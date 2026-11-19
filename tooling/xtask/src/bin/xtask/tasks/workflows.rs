// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Regenerates every checked-in GitHub Actions workflow, and the checks OpenTofu
//! requires of them. See [`xtask::workflows`].

use anyhow::Result;
use clap::Parser;
use xtask::workflows;

#[derive(Parser)]
pub struct Options {
    /// Report drift instead of writing. Exits non-zero when a generated
    /// workflow on disk differs from what this crate would emit.
    #[arg(long)]
    pub check: bool,
}

pub fn run(args: Options) -> Result<()> {
    workflows::generate(args.check)
}
