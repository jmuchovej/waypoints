use serde::{Deserialize, Serialize};

use super::{
    landmark::ContentFragment, landmark::LandmarkFragment, protochart::Protochart,
    terrain::TerrainFragment,
};
use crate::bundle::{Census, variant_name};

/// Everything one output carries. Order is the importer's and is kept.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fragments {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub landmarks: Vec<LandmarkFragment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protocharts: Vec<Protochart>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain: Vec<TerrainFragment>,
}

impl Fragments {
    pub fn is_empty(&self) -> bool {
        self.landmarks.is_empty() && self.protocharts.is_empty() && self.terrain.is_empty()
    }

    /// Add what these fragments use to a census: the serde tag of every
    /// content, reading, feature-kind, and terrain-kind variant present.
    /// Computed from output, never authored.
    pub fn tally(&self, census: &mut Census) {
        for landmark in &self.landmarks {
            for reading in landmark.readings.values() {
                census.readings.insert(variant_name(reading));
            }
            for block in &landmark.body {
                tally_content(block, census);
            }
        }
        for protochart in &self.protocharts {
            for feature in protochart.features.values() {
                census.feature_kinds.insert(variant_name(&feature.kind));
            }
        }
        for edge in &self.terrain {
            census.terrain_kinds.insert(variant_name(&edge.kind));
        }
    }
}

fn tally_content(block: &ContentFragment, census: &mut Census) {
    census.content.insert(variant_name(&block.content));
    for child in &block.children {
        tally_content(child, census);
    }
}
