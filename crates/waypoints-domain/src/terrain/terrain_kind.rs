use serde::{Deserialize, Serialize};
use specta::Type;

/// Terrain kinds which designate the relationship between two [`Marks`][M].
///
/// [M]: crate::mark::Mark
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TerrainKind {
    #[default]
    Child,
    Reference,
    Custom,
}
