use super::TerrainKind;
use crate::{Json, mark::MarkID};
use serde::{Deserialize, Serialize};
use specta::Type;
use typed_builder::TypedBuilder;

/// Graph edges representing relationships between blocks.
#[derive(Debug, Clone, PartialEq, TypedBuilder, Deserialize, Serialize, Type)]
#[builder(field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")))]
pub struct Terrain {
    /// The directed edge this block is coming from
    pub src_id: MarkID,

    /// The directed edge this block is going to
    pub dst_id: MarkID,

    /// What kind of edge is this?
    pub kind: TerrainKind,

    /// Extra metadata that belongs to this particular edge
    #[builder(default, via_mutators, mutators(
      pub fn with_extra(mut self, extra: Json) {
        self.extra = Some(extra);
      }
    ))]
    pub extra: Option<Json>,
}
