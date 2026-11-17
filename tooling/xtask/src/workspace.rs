// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use anyhow::{Context as _, Result};
use cargo_metadata::{Metadata, MetadataCommand, semver::Version};

/// The crate the shipped app is built from: the app group's own crate, whose
/// library target exists for the mobile bindings and whose contract is the
/// covenant rather than a Rust API.
pub const APP: &str = "Waypoints";

/// Workspace members and their manifests.
///
/// `--no-deps`: nothing here needs the resolved dependency graph, and skipping
/// it keeps this offline and an order of magnitude cheaper.
pub fn load() -> Result<Metadata> {
    MetadataCommand::new()
        .no_deps()
        .exec()
        .context("Failed to load cargo metadata")
}

/// The [`APP`] package's directory: `src-tauri/`.
pub fn app_dir(metadata: &Metadata) -> Result<PathBuf> {
    metadata
        .packages
        .iter()
        .find(|package| package.name.as_str() == APP)
        .and_then(|package| package.manifest_path.parent())
        .map(|dir| dir.as_std_path().to_path_buf())
        .with_context(|| format!("no `{APP}` package in this workspace"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release_groups;
    use std::{fs, path::Path};

    /// The app manifest is a projection of the group file: it declares its own
    /// version — a channel decision the library crates never share — and that
    /// version is the one the app's group declares.
    #[test]
    fn the_app_manifest_projects_the_declared_version() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/Cargo.toml");
        let manifest: toml::Table = fs::read_to_string(manifest).unwrap().parse().unwrap();

        let projected = manifest["package"]["version"]
            .as_str()
            .expect("the app must declare an explicit version, not inherit one");

        let declared = release_groups::load(&load().unwrap())
            .unwrap()
            .app_version()
            .unwrap();
        assert_eq!(declared.to_string(), projected);
    }
}
