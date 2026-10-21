//! mise: the config environments it composes, and the projection devenv
//! writes to `.config/mise/conf.d/tools.toml`.
//!
//! The projection is a committed, generated file: cheap to read and stable
//! to bake into generated output, unlike anything that needs a devenv
//! evaluation.

use anyhow::{Context as _, Result};
use derive_more::Display;
use std::fs;
use strum::EnumIter;

/// A mise config environment, as declared in `waypoints.mise.environments` and
/// materialised into `.config/mise/conf.d/tools.<name>.toml`.
///
/// mise does not validate `MISE_ENV`: a misspelled name loads the base config
/// and nothing else, so a job gets base-only tooling and fails on a missing
/// binary far from the typo. Naming environments through this enum is what
/// makes that unwritable; `every_variant_has_a_generated_config` keeps it
/// honest against the nix declaration.
///
/// The variants are the whole set — there is no union environment to exclude,
/// because `MISE_ENV` composes at selection and a job names the lanes it works
/// in. [`MiseEnv::select`] is what builds that list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, EnumIter)]
#[display(rename_all = "kebab-case")]
pub enum MiseEnv {
    Xtask,
    Test,
    /// Every formatter and every check — treefmt and the rest of the `hk`
    /// suite. Belongs on a job that runs a formatting or lint pass; the
    /// workflow audit is the one that does through mise, since
    /// `check-generated` reaches its tooling through nix.
    Devtools,
    BuildDesktop,
    BuildMobile,
    Website,
}

impl MiseEnv {
    /// The `MISE_ENV` value selecting all of `envs`. mise reads it as a
    /// comma-separated list and composes the named configs additively.
    pub fn select(envs: &[Self]) -> (&'static str, String) {
        (
            "MISE_ENV",
            envs.iter()
                .map(Self::to_string)
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

const MISE_TOML: &str = ".config/mise/conf.d/tools.toml";

/// The mise that writes `.config/mise/mise.lock`, read from the projection's
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use strum::IntoEnumIterator as _;

    /// mise will not validate `MISE_ENV`, so nothing but this test connects a
    /// variant to the config it selects. A renamed or dropped environment in
    /// `devenv.nix` leaves the variant naming a file that is not there, and a
    /// job selecting it silently gets base-only tooling.
    #[test]
    fn every_variant_has_a_generated_config() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for env in MiseEnv::iter() {
            let config = root.join(format!(".config/mise/conf.d/tools.{env}.toml"));
            assert!(
                config.exists(),
                "MiseEnv::{env:?} selects {}, which does not exist — has an environment been renamed in devenv.nix?",
                config.display(),
            );
        }
    }

    /// The whole point of the enum: a job names environments, and the value it
    /// produces is the comma-separated list mise composes.
    #[test]
    fn selecting_several_environments_joins_them() {
        assert_eq!(
            MiseEnv::select(&[MiseEnv::Xtask, MiseEnv::BuildMobile]),
            ("MISE_ENV", "xtask,build-mobile".to_owned()),
        );
    }
}
