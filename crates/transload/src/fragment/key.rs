use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

/// A source key: the identity an importer gives an entity, chosen by the
/// importer, stable across runs of the same source, and opaque to Waypoints.
///
/// Non-empty is the only rule. Which space a key is read in — Mark,
/// protochart, feature, or option — is decided by where it appears, never by
/// its spelling.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SourceKey(String);

/// The one way a source key can be malformed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a source key is a non-empty string")]
pub struct EmptySourceKey;

impl SourceKey {
    /// A key from any non-empty string.
    pub fn new(key: impl Into<String>) -> Result<Self, EmptySourceKey> {
        let key = key.into();
        if key.is_empty() {
            return Err(EmptySourceKey);
        }
        Ok(Self(key))
    }

    /// The key's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for SourceKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SourceKey {
    type Error = EmptySourceKey;

    fn try_from(key: String) -> Result<Self, Self::Error> {
        Self::new(key)
    }
}

impl TryFrom<&str> for SourceKey {
    type Error = EmptySourceKey;

    fn try_from(key: &str) -> Result<Self, Self::Error> {
        Self::new(key)
    }
}

impl std::str::FromStr for SourceKey {
    type Err = EmptySourceKey;

    fn from_str(key: &str) -> Result<Self, Self::Err> {
        Self::new(key)
    }
}

impl<'de> Deserialize<'de> for SourceKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let key = String::deserialize(deserializer)?;
        Self::new(key).map_err(serde::de::Error::custom)
    }
}

/// The reserved field. A Fragment entity ignores fields it does not define,
/// except this one: `id` is identity from the wrong side, and its presence
/// is refused on deserialization. Never serialized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NoId;

pub(crate) fn refuse_id<'de, D: Deserializer<'de>>(_: D) -> Result<NoId, D::Error> {
    Err(serde::de::Error::custom(
        "`id` is not a Fragment field — identity is minted by the core; name the entity by `src`",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_any_non_empty_string() {
        assert_eq!(SourceKey::new("page:foo").unwrap().as_str(), "page:foo");
        assert_eq!(SourceKey::new(""), Err(EmptySourceKey));
    }

    #[test]
    fn keys_round_trip_as_bare_strings() {
        let key: SourceKey = serde_json::from_str("\"block:7\"").unwrap();
        assert_eq!(serde_json::to_string(&key).unwrap(), "\"block:7\"");
        assert!(serde_json::from_str::<SourceKey>("\"\"").is_err());
    }
}
