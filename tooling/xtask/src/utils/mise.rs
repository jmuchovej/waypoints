//! Reads the mise projection devenv writes to `.config/mise.toml`.
//!
//! A committed, generated file: cheap to read and stable to bake into
//! generated output, unlike anything that needs a devenv evaluation.

use anyhow::{Context as _, Result, bail};
use std::fs;

const MISE_TOML: &str = ".config/mise.toml";
const FORMATTERS_VAR: &str = "WAYPOINTS_FORMATTERS";

/// The comma-separated specs of every tool that exists only to format —
/// treefmt and the formatters it drives — as devenv projects them into
/// `[vars].WAYPOINTS_FORMATTERS`. Already in the shape `MISE_DISABLE_TOOLS`
/// wants, so release jobs can skip installing any of it.
pub fn formatters() -> Result<String> {
    let text =
        fs::read_to_string(MISE_TOML).with_context(|| format!("failed to read {MISE_TOML}"))?;
    let table: toml::Table = text
        .parse()
        .with_context(|| format!("{MISE_TOML} is not valid TOML"))?;

    let value = table
        .get("vars")
        .and_then(|vars| vars.get(FORMATTERS_VAR))
        .and_then(toml::Value::as_str)
        .with_context(|| {
            format!(
                "{MISE_TOML} has no string `[vars].{FORMATTERS_VAR}` — has devenv regenerated it?"
            )
        })?;

    // An empty list means the projection lost its formatters; a workflow
    // generated from it would quietly install all of them again.
    if value.is_empty() {
        bail!("`[vars].{FORMATTERS_VAR}` in {MISE_TOML} is empty");
    }

    Ok(value.to_owned())
}
