//! IDs used for [`Charts`][C], and [`Features`][F] (and [`Glyphs` (Select Options)][S]).
//!
//! [C]: super::Chart
//! [F]: super::Feature
//! [S]: super::Select
use crate::error::{DomainError, DomainResult};
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
