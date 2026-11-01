use domain::Json;
use serde::{Deserialize, Serialize};

use super::key::{NoId, SourceKey, refuse_id};

/// A Terrain edge as an importer emits it: both ends by source key, the kind
/// structured rather than in its storage spelling, an optional ordinal among
/// parallel edges, and `extra`.
///
/// A child-kind line is annotative — nesting already carries the edge — and
/// exists only to attach `extra` to it, as in a Bundle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainFragment {
    pub src: SourceKey,
    pub dst: SourceKey,
    pub kind: TerrainKindFragment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Json>,
    #[serde(
        default,
        rename = "id",
        deserialize_with = "refuse_id",
        skip_serializing
    )]
    pub(crate) no_id: NoId,
}

/// The edge kinds a Fragment may carry. `TerrainKind::Reading` has no
/// counterpart on purpose: a reading edge is a derived index of a reading,
/// and deriving it is the core's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum TerrainKindFragment {
    #[default]
    Child,
    Reference,
    Custom,
}

impl TerrainFragment {
    /// An edge with no ordinal and no `extra`.
    pub fn new(src: SourceKey, dst: SourceKey, kind: TerrainKindFragment) -> Self {
        Self {
            src,
            dst,
            kind,
            order: None,
            extra: None,
            no_id: NoId,
        }
    }
}
