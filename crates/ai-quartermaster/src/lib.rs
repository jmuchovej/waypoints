//! quartermaster — model logistics for the Waypoints AI stack.
//!
//! Supplies, not semantics: this crate knows *which files* a model needs,
//! where they live on disk, and how to hand back a loaded session. It never
//! knows what a model computes — that is `instruments`' job.
//!
//! See `.agents/polaris-on-device-audio-ml.md` §6 for the crate topology.

pub mod cache;
pub mod hub;
pub mod registry;
pub mod session;

pub use registry::{License, ModelID, ModelSpec};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The requested model id is not present in the registry.
    #[error("unknown model: {0}")]
    UnknownModel(String),
    /// Download or filesystem failure while materializing model artifacts.
    #[error("failed to fetch model artifacts: {0}")]
    Fetch(String),
}
