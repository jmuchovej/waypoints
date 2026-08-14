pub(crate) mod markdown;
pub use markdown::{MarkdownImporter, WaypointsMarkdown};

use domain::{Mark, Reading, Terrain};

/// The result of importing a document into Waypoints' domain model.
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// All marks: the landmark (first) followed by its content children.
    pub marks: Vec<Mark>,
    /// Terrain edges wiring the landmark to its content children.
    pub terrains: Vec<Terrain>,
    /// Readings keyed by feature **name** (not ID).
    ///
    /// The importer can't resolve feature names to [`FeatureID`]s — that
    /// requires the persisted chart definition from the DB. The consumer
    /// (e.g. a migration) is responsible for resolving names to IDs and
    /// attaching these readings to the landmark mark before insertion.
    pub readings: Vec<(String, Reading)>,
}
