use derive_more::{Debug, Display};
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// Unique identifier for a [`Terrain`][super::Terrain] edge.
///
/// Each edge has its own stable identity independent of its endpoints. This
/// enables a true multigraph: multiple edges between the same `(src, dst)`
/// pair — even of the same [`TerrainKind`][super::TerrainKind] — can coexist
/// as distinct rows, each with their own `TerrainID`. Without this, the
/// primary key would need to encode every dimension that could vary per
/// edge (kind, feature attribution, order, etc.), which bakes assumptions
/// into the PK itself and makes future extensions painful.
///
/// Generated as a random [`Uuid::new_v4`] on edge creation. Never changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Type)]
#[display("{}", self.inner().to_string().to_lowercase())]
#[debug("TerrainID({})", self.inner().to_string().to_lowercase())]
#[serde(transparent)]
pub struct TerrainID(pub Uuid);

impl TerrainID {
    /// Create a new, random [`TerrainID`] (a [`Uuid::new_v4`]).
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Retrieve the inner [`Uuid`] within the [`TerrainID`].
    pub fn inner(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for TerrainID {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for TerrainID {
    fn default() -> Self {
        Self::new()
    }
}
