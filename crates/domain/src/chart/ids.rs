//! IDs used for [`Charts`][C], and [`Features`][F] (and [`Glyphs` (Select Options)][S]).
//!
//! [C]: super::Chart
//! [F]: super::Feature
//! [S]: super::Select
use crate::{DomainError, DomainResult};
use derive_more::{Debug, Display, FromStr};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

const BASE36ID_LEN: usize = 4;

/// Base-36 encoded identifier newtype. (Currently 4-character limit.)
///
/// This is the core type that [`ChartID`], [`FeatureID`], and [`GlyphID`] wrap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Type)]
#[display("{_0}")]
#[debug("b36::{_0}")]
#[serde(transparent)]
pub struct Base36ID(pub String);

impl Base36ID {
    /// Generate a new random N-character base-36 ID
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let mut id = String::with_capacity(BASE36ID_LEN);

        const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        for _ in 0..BASE36ID_LEN {
            let idx = rng.random_range(0..CHARS.len());
            id.push(CHARS[idx] as char);
        }

        Self(id)
    }

    /// A deterministic ID from a stable slug — never a display name, which may
    /// change. For builtins, whose IDs must agree across stores and launches.
    pub fn derived(key: &str) -> Self {
        use sha2::{Digest, Sha256};

        const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let digest = Sha256::digest(key.as_bytes());
        let mut prefix = [0u8; 8];
        prefix.copy_from_slice(&digest[..8]);
        let mut value = u64::from_be_bytes(prefix) % 36u64.pow(BASE36ID_LEN as u32);

        let mut id = [b'0'; BASE36ID_LEN];
        for slot in id.iter_mut().rev() {
            *slot = CHARS[(value % 36) as usize];
            value /= 36;
        }
        Self(String::from_utf8_lossy(&id).into_owned())
    }

    /// Parse a base-36 ID from a string
    pub fn parse(s: &str) -> DomainResult<Base36ID> {
        use DomainError::{Base36IDInvalidCharacters, Base36IDLengthMismatch};

        if s.starts_with("builtin.") {
            return Ok(Self(s.to_lowercase()));
        }

        if s.len() != BASE36ID_LEN {
            return Err(Base36IDLengthMismatch {
                expected: BASE36ID_LEN,
                found: s.len(),
            });
        }

        // Validate base-36 characters
        if !s.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(Base36IDInvalidCharacters);
        }

        Ok(Self(s.to_lowercase()))
    }

    /// Get the raw string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Base36ID {
    fn default() -> Self {
        Self::new()
    }
}

impl std::str::FromStr for Base36ID {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Unique identifier for a [`Chart`][C].
///
/// [ChartIDs][`ChartID`] are 4-character base-36 identifiers for custom [`Charts`][C],
/// or special `"builtin.*"` identifiers for built-in chart types.
///
/// [C]: super::Chart
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Type, bedrock::Scalar,
)]
#[display("{}", self.name())]
#[debug("ChartID({})", self.name())]
#[serde(transparent)]
pub struct ChartID(pub String);

impl ChartID {
    /// Generate a new random 4-character [`Base36ID`].
    pub fn new() -> Self {
        Self(Base36ID::new().to_string())
    }

    /// Create a builtin [`ChartID`] with `"builtin.{name}"` prefix.
    pub fn builtin(name: &str) -> Self {
        Self(format!("builtin.{name}"))
    }

    /// Check if this [`ChartID`] represents a builtin type.
    pub fn is_builtin(&self) -> bool {
        self.0.starts_with("builtin.")
    }

    /// Get the display name (without `"builtin.{name}"` prefix).
    pub fn name(&self) -> &str {
        self.0.strip_prefix("builtin.").unwrap_or(self.0.as_str())
    }
}

impl Default for ChartID {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ChartID> for String {
    fn from(id: ChartID) -> Self {
        id.0
    }
}

impl std::str::FromStr for ChartID {
    type Err = DomainError;

    /// Create [`ChartID`] from string (validates 4-character base-36)
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Base36ID::parse(s)?.to_string()))
    }
}

impl TryFrom<String> for ChartID {
    type Error = DomainError;

    fn try_from(value: String) -> DomainResult<Self> {
        value.parse::<ChartID>()
    }
}

/// Unique identifier for a [`Feature`][F] definition within a [`Chart`][C].
///
/// [`FeatureIDs`][FeatureID] are 4-character base-36 identifiers used to reference
/// specific features within a [`Chart`][C] and to store field values on [`Landmarks`][L].
///
/// [C]: super::Chart
/// [F]: super::Feature
/// [L]: crate::mark::Landmark
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr, Type)]
#[display("{_0}")]
#[debug("FeatureID({_0})")]
#[serde(transparent)]
pub struct FeatureID(pub Base36ID);

impl FeatureID {
    /// Generate a new random 4-character base-36 field ID
    pub fn new() -> Self {
        Self(Base36ID::new())
    }

    /// See [`Base36ID::derived`].
    pub fn derived(key: &str) -> Self {
        Self(Base36ID::derived(key))
    }
}

impl Default for FeatureID {
    fn default() -> Self {
        Self::new()
    }
}

impl From<FeatureID> for String {
    fn from(id: FeatureID) -> Self {
        id.0.as_str().to_string()
    }
}

/// Unique identifier for a select option within a [`Feature`][F].
///
/// [F]: super::Feature
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr, Type)]
#[display("{_0}")]
#[debug("GlyphID({_0})")]
#[serde(transparent)]
pub struct GlyphID(pub Base36ID);

impl GlyphID {
    /// Generate a new random 4-character base-36 option ID
    pub fn new() -> Self {
        Self(Base36ID::new())
    }

    /// See [`Base36ID::derived`].
    pub fn derived(key: &str) -> Self {
        Self(Base36ID::derived(key))
    }
}

impl Default for GlyphID {
    fn default() -> Self {
        Self::new()
    }
}

impl From<GlyphID> for String {
    fn from(id: GlyphID) -> Self {
        id.0.as_str().to_string()
    }
}

#[cfg(test)]
mod derived_tests {
    use super::*;

    #[test]
    fn a_derived_id_is_deterministic_and_well_formed() {
        let id = Base36ID::derived("authors");
        assert_eq!(id, Base36ID::derived("authors"));
        assert_eq!(Base36ID::parse(&id.0).expect("valid base-36"), id);
        assert_ne!(id, Base36ID::derived("summary"));
    }
}
