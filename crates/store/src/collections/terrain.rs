use chrono::NaiveDateTime;
use domain::{
    mark::MarkID,
    terrain::{Terrain, TerrainID, TerrainKind},
};
use serde_json::Value;

use crate::entities::{
    terrain_kind_to_string as kind_to_string, terrain_string_to_kind as string_to_kind,
};

/// Bedrock-backed storage row for a [`Terrain`] (graph edge).
///
/// `kind` is stored as a flat string (e.g. `"child"`, `"reading::builtin.page::a1b2"`)
/// so it can be indexed and filtered without JSON parsing. `src_id`/`dst_id`/`kind`
/// are individually indexed to support the common query patterns.
///
/// No sync columns — sync was removed.
#[derive(Debug, Clone, PartialEq, bedrock::Collection)]
pub struct TerrainRow {
    #[bedrock(id)]
    pub id: TerrainID,

    #[bedrock(index)]
    pub src_id: MarkID,

    #[bedrock(index)]
    pub dst_id: MarkID,

    /// Flattened kind string. See [`kind_to_string`] / [`string_to_kind`].
    #[bedrock(index)]
    pub kind: String,

    /// Fractional index — meaningful for Child and Reading edges. The domain
    /// field is `Terrain.order`; the column is `sequence` to avoid the SQL
    /// reserved word.
    pub sequence: Option<String>,

    pub extra: Option<Value>,

    /// Soft-delete tombstone.
    pub deleted_at: Option<NaiveDateTime>,
}

impl From<TerrainRow> for Terrain {
    fn from(row: TerrainRow) -> Self {
        let kind = string_to_kind(&row.kind);

        let mut terrain = Terrain::builder()
            .with_src_id(row.src_id)
            .with_dst_id(row.dst_id)
            .with_kind(kind)
            .build();

        terrain.id = row.id;
        terrain.order = row.sequence;
        terrain.extra = row.extra;

        terrain
    }
}

impl TerrainRow {
    pub fn from_terrain(terrain: &Terrain) -> Self {
        Self {
            id: terrain.id,
            src_id: terrain.src_id,
            dst_id: terrain.dst_id,
            kind: kind_to_string(&terrain.kind),
            sequence: terrain.order.clone(),
            extra: terrain.extra.clone(),
            deleted_at: None,
        }
    }
}
