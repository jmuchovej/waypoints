//! Reads the mise projection devenv writes to `.config/mise.toml`.
//!
//! A committed, generated file: cheap to read and stable to bake into
//! generated output, unlike anything that needs a devenv evaluation.

use anyhow::{Context as _, Result, bail};
use std::fs;

const MISE_TOML: &str = ".config/mise.toml";
const FORMATTERS_VAR: &str = "WAYPOINTS_FORMATTERS";

/// One `[vars]` entry of the projection.
///
/// Empty is an error rather than a default: every caller bakes the value into a
/// generated workflow, where an empty string means something quietly different
/// from what was intended — every formatter reinstalled, or mise floating to
/// whatever released last.
fn var(name: &str) -> Result<String> {
    let text =
        fs::read_to_string(MISE_TOML).with_context(|| format!("failed to read {MISE_TOML}"))?;
    let table: toml::Table = text
        .parse()
        .with_context(|| format!("{MISE_TOML} is not valid TOML"))?;

    let value = table
        .get("vars")
        .and_then(|vars| vars.get(name))
        .and_then(toml::Value::as_str)
        .with_context(|| {
            format!("{MISE_TOML} has no string `[vars].{name}` — has devenv regenerated it?")
        })?;

    if value.is_empty() {
        bail!("`[vars].{name}` in {MISE_TOML} is empty");
    }

    Ok(value.to_owned())
}

/// The comma-separated specs of every tool that exists only to format —
/// treefmt and the formatters it drives — as devenv projects them into
/// `[vars].WAYPOINTS_FORMATTERS`. Already in the shape `MISE_DISABLE_TOOLS`
/// wants, so release jobs can skip installing any of it.
pub fn formatters() -> Result<String> {
    var(FORMATTERS_VAR)
}

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
