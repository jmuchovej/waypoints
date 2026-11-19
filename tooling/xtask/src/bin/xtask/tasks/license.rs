// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Holds the tree to the licensing policy. The checks are [`xtask::license`]'s;
//! this runs them and prints what they found: `check licenses` and `generate
//! notices`.

use anyhow::{Context as _, Result, bail};
use std::fs;
use xtask::{license, workspace};

/// Checks manifests, headers, layering, license texts, and the third-party
/// registry.
pub fn check() -> Result<()> {
    let root = workspace::load()?.workspace_root.into_std_path_buf();
    let report = license::check_headers(&root)?;

    for finding in &report.findings {
        eprintln!("{finding}");
    }
    if !report.findings.is_empty() {
        bail!("{} licensing finding(s)", report.findings.len());
    }
    println!(
        "{} crates and the tree agree with {} (SPDX list {})",
        report.crates,
        license::POLICY,
        spdx::identifiers::VERSION,
    );
    Ok(())
}

/// Regenerates `THIRD-PARTY-NOTICES.md` from the registry and the lockfiles.
pub fn notices() -> Result<()> {
    let root = workspace::load()?.workspace_root.into_std_path_buf();
    let contents = license::generate_notices(&root)?;
    let path = root.join(license::NOTICES_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, contents).with_context(|| format!("Failed to write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}
