use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};

/// Terrain kinds which designate the relationship between two [`Marks`][M].
///
/// [M]: crate::mark::Mark
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[frb(non_opaque, unignore)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TerrainKind {
    #[default]
    Child,
    Reference,
    Custom,
}
