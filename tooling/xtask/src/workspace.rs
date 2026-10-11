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

    /// [`version`] reads the app package, which now carries its own version:
    /// the app's is a channel decision, while the library crates move at their
    /// own cadence, so the two cannot share one field.
    #[test]
    fn the_app_declares_its_own_version() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/Cargo.toml");
        let manifest: toml::Table = fs::read_to_string(manifest).unwrap().parse().unwrap();

        let declared = manifest["package"]["version"]
            .as_str()
            .expect("the app must declare an explicit version, not inherit one");

        assert_eq!(version().unwrap().to_string(), declared);
    }
}
