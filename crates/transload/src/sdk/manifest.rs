use super::{
    importer::{Importer, Manifested},
    schema,
};
use crate::abi::{Authored, Manifest};

/// Why an authored manifest could not be served.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// The TOML did not parse as a manifest: a missing required field, a
    /// field the manifest does not define, or a type mismatch. The message
    /// names the field.
    #[error("the manifest is not valid: {0}")]
    Invalid(String),
    /// A required text field was present but empty.
    #[error("the manifest's `{0}` must not be empty")]
    Empty(&'static str),
}

/// Parse the author's TOML. Unknown fields are refused; required fields are
/// required; empty identity fields are refused.
pub fn parse(authored: &str) -> Result<Authored, ManifestError> {
    let authored: Authored = toml::from_str(authored)
        .map_err(|error| ManifestError::Invalid(error.message().to_string()))?;
    if authored.id.is_empty() {
        return Err(ManifestError::Empty("id"));
    }
    if authored.name.is_empty() {
        return Err(ManifestError::Empty("name"));
    }
    if authored.namespace.is_empty() {
        return Err(ManifestError::Empty("namespace"));
    }
    if authored.source.kinds.is_empty() {
        return Err(ManifestError::Empty("source.kinds"));
    }
    Ok(authored)
}

/// The manifest for a plugin: parsed, checked, and stamped.
pub fn build<T: Importer + Manifested>() -> Result<Manifest, ManifestError> {
    let authored = parse(T::AUTHORED)?;
    let options = schema::options::<T::Config>();
    Ok(Manifest::stamp(authored, T::VERSION, options))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
id = "echo"
name = "Echo"
namespace = "so.waypoints.importer.echo"

[icon]
identifier = "system:emoji:loudspeaker"

[source]
kinds = ["directory"]
markers = ["echo.toml"]
extensions = [".txt"]
"#;

    #[test]
    fn a_valid_manifest_parses() {
        let authored = parse(GOOD).unwrap();
        assert_eq!(authored.id, "echo");
        assert_eq!(authored.source.markers, ["echo.toml"]);
    }

    #[test]
    fn a_missing_required_field_names_itself() {
        let error = parse(&GOOD.replace("name = \"Echo\"\n", "")).unwrap_err();
        assert!(error.to_string().contains("name"), "{error}");
    }

    #[test]
    fn an_undefined_field_names_itself() {
        let error = parse(&format!("version = \"1.0.0\"\n{GOOD}")).unwrap_err();
        assert!(error.to_string().contains("version"), "{error}");
    }

    #[test]
    fn an_empty_identity_is_refused() {
        let error = parse(&GOOD.replace("id = \"echo\"", "id = \"\"")).unwrap_err();
        assert_eq!(error, ManifestError::Empty("id"));
    }
}
