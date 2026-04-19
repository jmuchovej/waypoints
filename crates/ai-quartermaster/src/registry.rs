//! The model registry: which artifacts exist, where they come from, and what
//! license rides along. The ADR's licensing constraints (§9) are enforced
//! here — a model whose license cannot be expressed as a [`License`] variant
//! cannot be registered.

use derive_more::Display;
use serde::{Deserialize, Serialize};

/// Identifies a model in the registry (e.g. `"parakeet-tdt-0.6b-v3"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
#[serde(transparent)]
pub struct ModelID(pub String);

/// Licenses Waypoints is willing to ship. Deliberately closed: CC-BY-NC and
/// vendor "open model" licenses have no variant, so they cannot sneak in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum License {
    MIT,
    Apache2,
    CcBy4,
}

/// Everything quartermaster needs to materialize a model: where it lives on
/// the Hub, which files to pull, and what license rides along.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub id: ModelID,
    /// Hugging Face repo (e.g. `"nvidia/parakeet-tdt-0.6b-v3"`).
    pub repo: String,
    /// Pinned revision — never a branch name; downloads must be reproducible.
    pub revision: String,
    /// Files to fetch from the repo.
    pub files: Vec<String>,
    pub license: License,
}
