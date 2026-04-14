use crate::chart::{ChartID, FeatureID};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Terrain kinds which designate the relationship between two [`Marks`][M].
///
/// [M]: crate::mark::Mark
///
/// # Serde format
///
/// Uses adjacent tagging (`{ "type": ..., "value": ... }`) so that the
/// parameterized [`Reading`][Self::Reading] variant can carry its
/// `(ChartID, FeatureID)` payload cleanly. Internally-tagged enums only
/// support newtype or struct variants, but tuple variants are more
/// ergonomic for Rust callers.
///
/// ```json
/// // Child    → { "type": "child" }
/// // Reading  → { "type": "reading", "value": ["builtin.meeting", "a1b2"] }
/// ```
///
/// # DB format
///
/// When persisted to the `terrains.kind` column we flatten to a single
/// string: `"child"`, `"reference"`, `"custom"`, or
/// `"reading::{chart}::{feature}"`. See the helpers in
/// `store::query::terrain` / `store::entities::terrain_orm` for the
/// bidirectional conversion.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum TerrainKind {
    #[default]
    Child,
    Reference,
    Custom,
    /// A [`Reading`][crate::mark::Reading] of kind
    /// [`Reference`][crate::chart::FeatureKind::Reference] materialized as a
    /// graph edge. The `ChartID` and `FeatureID` identify *which* Reference
    /// feature on *which* Chart the edge represents — enabling queries like
    /// "all backlinks into this landmark attributed to the `attendees`
    /// feature of a `Meeting`" without scanning every landmark's readings
    /// JSON.
    Reading(ChartID, FeatureID),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn simple_variants_roundtrip_through_serde_json() {
        for kind in [
            TerrainKind::Child,
            TerrainKind::Reference,
            TerrainKind::Custom,
        ] {
            let encoded = serde_json::to_string(&kind).unwrap();
            let decoded: TerrainKind = serde_json::from_str(&encoded).unwrap();
            assert_eq!(kind, decoded, "roundtrip failed for {kind:?}");
        }
    }

    #[test]
    fn child_serializes_with_adjacent_tag() {
        let json = serde_json::to_value(TerrainKind::Child).unwrap();
        assert_eq!(json, serde_json::json!({"type": "child"}));
    }

    #[test]
    fn reading_serializes_with_tuple_value() {
        // Adjacent tagging — tuple variants serialize their payload as an
        // array under the `value` key. Both ChartID and FeatureID are
        // `#[serde(transparent)]`, so they appear as bare strings in the
        // output.
        let chart = ChartID::builtin("meeting");
        let feature = FeatureID::from_str("a1b2").unwrap();
        let kind = TerrainKind::Reading(chart, feature);

        let json = serde_json::to_value(&kind).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "reading",
                "value": ["builtin.meeting", "a1b2"],
            })
        );
    }

    #[test]
    fn reading_roundtrips_through_serde_json() {
        let chart = ChartID::builtin("meeting");
        let feature = FeatureID::from_str("a1b2").unwrap();
        let kind = TerrainKind::Reading(chart.clone(), feature.clone());

        let encoded = serde_json::to_string(&kind).unwrap();
        let decoded: TerrainKind = serde_json::from_str(&encoded).unwrap();
        assert_eq!(kind, decoded);

        // Verify the payload preserved both IDs exactly — including the
        // `"builtin."` prefix that ChartID::Display would strip.
        match decoded {
            TerrainKind::Reading(c, f) => {
                assert_eq!(c, chart);
                assert_eq!(f, feature);
            }
            other => panic!("expected Reading variant, got {other:?}"),
        }
    }

    #[test]
    fn default_is_child() {
        assert_eq!(TerrainKind::default(), TerrainKind::Child);
    }
}
