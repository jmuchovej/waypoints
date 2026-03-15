#[allow(clippy::module_inception)]
pub mod terrain;
pub use terrain::Terrain;
pub mod terrain_kind;
pub use terrain_kind::TerrainKind;

pub mod builders {
    use super::terrain;
    pub use terrain::TerrainBuilder;
}
