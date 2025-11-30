use super::TerrainKind;
use crate::{Json, mark::MarkID};
use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// Graph edges representing relationships between blocks.
#[derive(Debug, Clone, PartialEq, TypedBuilder, Deserialize, Serialize)]
#[builder(
  builder_type(attributes(#[frb(ignore)])),
  field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")),
)]
#[frb(non_opaque, unignore)]
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
