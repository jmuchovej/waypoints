use super::{TerrainID, TerrainKind};
use crate::{Json, mark::MarkID};
use serde::{Deserialize, Serialize};
use specta::Type;
use typed_builder::TypedBuilder;

/// Graph edges representing relationships between blocks.
///
/// Each edge has a stable [`TerrainID`] that is independent of its endpoints.
/// This enables a true multigraph: multiple edges — including multiple edges
/// of the same `kind` — can exist between the same `(src_id, dst_id)` pair,
/// distinguished by their `id`.
#[derive(Debug, Clone, PartialEq, TypedBuilder, Deserialize, Serialize, Type)]
#[builder(field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")))]
pub struct Terrain {
    /// Stable edge identity. Generated on creation, never changes. Use this
    /// to address a specific edge unambiguously (e.g., when deleting or
    /// updating a single edge among several of the same kind between the
    /// same endpoints).
    #[builder(default, setter(skip))]
    pub id: TerrainID,

    /// The directed edge this block is coming from
    pub src_id: MarkID,

    /// The directed edge this block is going to
    pub dst_id: MarkID,

    /// What kind of edge is this?
    pub kind: TerrainKind,

    /// Lexicographic ordering string (only meaningful for certain edge kinds like `Child`)
    pub order: Option<String>,

    /// Extra metadata that belongs to this particular edge
    #[builder(default, via_mutators, mutators(
      pub fn with_extra(mut self, extra: Json) {
        self.extra = Some(extra);
      }
    ))]
    #[specta(type = Option<specta_typescript::Unknown>)]
    pub extra: Option<Json>,
}
