//! Reads the mise projection devenv writes to `.config/mise.toml`.
//!
//! A committed, generated file: cheap to read and stable to bake into
//! generated output, unlike anything that needs a devenv evaluation.

use anyhow::{Context as _, Result};
use std::fs;

const MISE_TOML: &str = ".config/mise.toml";

/// The mise that writes `.config/mise.lock`, read from the projection's
/// `min_version`, so CI installs the same one.
///
/// Left unpinned, `mise-action` takes whatever released most recently, which
/// makes a committed lockfile only as reproducible as an upstream release
/// schedule — mise 2026.9.16 tightened SLSA provenance and broke a nightly
/// hours after publishing, with nothing changed here.
///
/// `min_version` is a floor to mise and an exact pin to us. That narrowing is
/// deliberate: CI should run the mise the lockfile was written by, not merely
/// one no older. mise also accepts a `{ hard, soft }` table there; devenv only
/// ever writes the string form, so anything else is a malformed projection
/// rather than a case to interpret.
pub fn version() -> Result<String> {
    let text =
        fs::read_to_string(MISE_TOML).with_context(|| format!("failed to read {MISE_TOML}"))?;
    let table: toml::Table = text
        .parse()
        .with_context(|| format!("{MISE_TOML} is not valid TOML"))?;

    let value = table.get("min_version").with_context(|| {
        format!("{MISE_TOML} has no `min_version` — has devenv regenerated it?")
    })?;

    value
        .as_str()
        .map(str::to_owned)
        .with_context(|| format!("`min_version` in {MISE_TOML} is {value}, not a version string"))
}
