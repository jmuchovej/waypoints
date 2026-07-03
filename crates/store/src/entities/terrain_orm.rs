use domain::chart::{ChartID, FeatureID};
use domain::terrain::{Terrain, TerrainID, TerrainKind};
use sea_orm::{
    ActiveModelBehavior, ActiveValue::Set, DeriveEntityModel, DerivePrimaryKey, DeriveRelation,
    EntityTrait, EnumIter, PrimaryKeyTrait, prelude::Json,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

/// Graph edges representing relationships between blocks.
///
/// Primary key is `id` (a random UUID per edge). `src_id`, `dst_id`, and
/// `kind` are individually indexed to support the common query patterns
/// (children of a parent, parent(s) of a child, backlinks by kind). This
/// layout supports a true multigraph — multiple edges of the same kind
/// between the same `(src, dst)` pair can coexist, each with their own
/// `id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "terrains")]
pub struct Model {
    /// Stable edge identity. Generated on insert.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    /// The directed edge this block is coming from
    #[sea_orm(indexed)]
    pub src_id: Uuid,

    /// The directed edge this block is going to
    #[sea_orm(indexed)]
    pub dst_id: Uuid,

    /// What kind of edge is this? Flattened to a string:
    /// - `"child"`, `"reference"`, `"custom"` for simple kinds
    /// - `"reading::{chart_id}::{feature_id}"` for
    ///   [`TerrainKind::Reading`][domain::terrain::TerrainKind::Reading]
    #[sea_orm(indexed)]
    pub kind: String,

    /// Lexicographic ordering string (only meaningful for certain edge kinds).
    /// Stored as `sequence` — ORDER is a reserved word.
    #[sea_orm(column_name = "sequence")]
    pub order: Option<String>,

    /// Extra metadata that belongs to this particular edge
    pub extra: Option<Json>,

    /// Sync row clock: packed HLC of the last sync-visible write.
    /// 0 = never touched by sync (any incoming op dominates).
    pub updated_hlc: i64,
    /// Sync row clock: device that produced the winning write.
    pub updated_by_device: Option<Uuid>,
}

use super::{MarkColumn, Marks};
use Column::{DstId, SrcId};

#[derive(Debug, Copy, Clone, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "Marks", from = "SrcId", to = "MarkColumn::Id")]
    SrcMark,
    #[sea_orm(belongs_to = "Marks", from = "DstId", to = "MarkColumn::Id")]
    DstMark,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

// ── Kind ⇄ String ──────────────────────────────────────────────────────
//
// The DB column stores a flat string so it can be indexed and filtered
// without JSON parsing. The domain type is a proper enum. These helpers
// bridge the two representations.

/// Flatten a [`TerrainKind`] to its DB-persisted string form.
///
/// NOTE: `ChartID`'s `Display` impl is lossy — it strips the `"builtin."`
/// prefix for human readability, which would make `ChartID::builtin("page")`
/// stringify to `"page"` and then (ambiguously) parse back to a custom 4-char
/// chart. We bypass it by accessing the inner `String` directly, so
/// `"builtin.page"` round-trips losslessly through `string_to_kind`.
/// `FeatureID`'s `Display` is not lossy so we use it normally.
pub fn kind_to_string(kind: &TerrainKind) -> String {
    match kind {
        TerrainKind::Child => "child".to_string(),
        TerrainKind::Reference => "reference".to_string(),
        TerrainKind::Custom => "custom".to_string(),
        TerrainKind::Reading(chart, feature) => {
            format!("reading::{}::{}", chart.0, feature)
        }
    }
}

/// Parse a DB-persisted kind string back into a [`TerrainKind`].
///
/// Unknown prefixes fall back to [`TerrainKind::Custom`] — lossy but safe.
/// Malformed `reading::*` strings also degrade to `Custom`; in practice
/// this shouldn't happen because `kind_to_string` is the only producer.
pub fn string_to_kind(s: &str) -> TerrainKind {
    if let Some(rest) = s.strip_prefix("reading::") {
        if let Some((chart_str, feature_str)) = rest.split_once("::") {
            if let (Ok(chart), Ok(feature)) = (
                ChartID::from_str(chart_str),
                FeatureID::from_str(feature_str),
            ) {
                return TerrainKind::Reading(chart, feature);
            }
        }
        // Malformed reading string — fall through to Custom
        return TerrainKind::Custom;
    }
    match s {
        "child" => TerrainKind::Child,
        "reference" => TerrainKind::Reference,
        _ => TerrainKind::Custom,
    }
}

impl From<Model> for Terrain {
    fn from(model: Model) -> Self {
        let kind = string_to_kind(&model.kind);

        let mut terrain = Terrain::builder()
            .with_src_id(model.src_id.into())
            .with_dst_id(model.dst_id.into())
            .with_kind(kind)
            .build();

        terrain.id = model.id.into();
        terrain.order = model.order;
        terrain.extra = model.extra;

        terrain
    }
}

impl From<Terrain> for ActiveModel {
    fn from(terrain: Terrain) -> Self {
        ActiveModel {
            id: Set(terrain.id.inner()),
            src_id: Set(terrain.src_id.inner()),
            dst_id: Set(terrain.dst_id.inner()),
            kind: Set(kind_to_string(&terrain.kind)),
            order: Set(terrain.order),
            extra: Set(terrain.extra),
            // Row clocks are managed by the sync layer, never by the
            // domain conversion.
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(chart_spec: &str, feature_spec: &str) -> TerrainKind {
        TerrainKind::Reading(
            ChartID::from_str(chart_spec).unwrap(),
            FeatureID::from_str(feature_spec).unwrap(),
        )
    }

    // ── kind_to_string ────────────────────────────────────────────────

    #[test]
    fn kind_to_string_simple_variants() {
        assert_eq!(kind_to_string(&TerrainKind::Child), "child");
        assert_eq!(kind_to_string(&TerrainKind::Reference), "reference");
        assert_eq!(kind_to_string(&TerrainKind::Custom), "custom");
    }

    #[test]
    fn kind_to_string_reading_builtin() {
        // Critical case: `"builtin.page"` must survive verbatim. `ChartID`'s
        // `Display` would strip the prefix, which is why we bypass it.
        let kind = reading("builtin.page", "a1b2");
        assert_eq!(kind_to_string(&kind), "reading::builtin.page::a1b2");
    }

    #[test]
    fn kind_to_string_reading_custom_chart() {
        // 4-char base-36 ChartID (no "builtin." prefix)
        let kind = reading("c3d4", "e5f6");
        assert_eq!(kind_to_string(&kind), "reading::c3d4::e5f6");
    }

    // ── string_to_kind ────────────────────────────────────────────────

    #[test]
    fn string_to_kind_simple_variants() {
        assert_eq!(string_to_kind("child"), TerrainKind::Child);
        assert_eq!(string_to_kind("reference"), TerrainKind::Reference);
        assert_eq!(string_to_kind("custom"), TerrainKind::Custom);
    }

    #[test]
    fn string_to_kind_reading_builtin() {
        let kind = string_to_kind("reading::builtin.page::a1b2");
        match kind {
            TerrainKind::Reading(chart, feature) => {
                assert_eq!(chart.0, "builtin.page");
                assert_eq!(feature.to_string(), "a1b2");
            }
            other => panic!("expected Reading, got {other:?}"),
        }
    }

    #[test]
    fn string_to_kind_reading_custom_chart() {
        let kind = string_to_kind("reading::c3d4::e5f6");
        match kind {
            TerrainKind::Reading(chart, feature) => {
                assert_eq!(chart.0, "c3d4");
                assert_eq!(feature.to_string(), "e5f6");
            }
            other => panic!("expected Reading, got {other:?}"),
        }
    }

    #[test]
    fn string_to_kind_unknown_prefix_degrades_to_custom() {
        assert_eq!(string_to_kind("gibberish"), TerrainKind::Custom);
        assert_eq!(string_to_kind(""), TerrainKind::Custom);
        assert_eq!(
            string_to_kind("READING::UPPERCASE::foo"),
            TerrainKind::Custom
        );
    }

    #[test]
    fn string_to_kind_malformed_reading_degrades_to_custom() {
        // Missing the feature segment entirely
        assert_eq!(string_to_kind("reading::builtin.page"), TerrainKind::Custom);
        // Feature ID isn't valid base-36 (too long)
        assert_eq!(
            string_to_kind("reading::builtin.page::way-too-long-feature-id"),
            TerrainKind::Custom
        );
        // Chart ID has disallowed characters (not builtin, not 4-char base-36)
        assert_eq!(string_to_kind("reading::!!::a1b2"), TerrainKind::Custom);
    }

    // ── Roundtrip symmetry ────────────────────────────────────────────

    #[test]
    fn roundtrip_all_simple_variants() {
        for kind in [
            TerrainKind::Child,
            TerrainKind::Reference,
            TerrainKind::Custom,
        ] {
            let s = kind_to_string(&kind);
            let decoded = string_to_kind(&s);
            assert_eq!(kind, decoded, "roundtrip failed for {kind:?}");
        }
    }

    #[test]
    fn roundtrip_reading_builtin() {
        let kind = reading("builtin.objective", "stat");
        let s = kind_to_string(&kind);
        let decoded = string_to_kind(&s);
        assert_eq!(kind, decoded);
    }

    #[test]
    fn roundtrip_reading_custom() {
        let kind = reading("c3d4", "e5f6");
        let s = kind_to_string(&kind);
        let decoded = string_to_kind(&s);
        assert_eq!(kind, decoded);
    }
}
