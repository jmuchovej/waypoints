use anyhow::{Context as _, Result};
use cargo_metadata::{Metadata, MetadataCommand, semver::Version};

/// The crate the shipped app is built from.
const APP: &str = "Waypoints";

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

/// The version the app ships as, read from the resolved [`APP`] package.
pub fn version() -> Result<Version> {
    let metadata = load()?;

    metadata
        .packages
        .iter()
        .find(|package| package.name.as_str() == APP)
        .map(|package| package.version.clone())
        .with_context(|| format!("no `{APP}` package in this workspace"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    /// [`version`] reads the resolved package, which matches the workspace
    /// declaration only while the app inherits it.
    #[test]
    fn the_app_version_matches_the_workspace_declaration() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest: toml::Table = fs::read_to_string(root.join("Cargo.toml"))
            .unwrap()
            .parse()
            .unwrap();

        let declared = manifest["workspace"]["package"]["version"]
            .as_str()
            .unwrap();

        assert_eq!(version().unwrap().to_string(), declared);
    }
}
