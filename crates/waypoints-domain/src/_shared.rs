//! Shared utilities and types for the waypoints domain.

pub type Json = serde_json::Value;
pub type DateTime = chrono::NaiveDateTime;
pub type Uuid = uuid::Uuid;

pub mod color;
pub mod error;

pub use color::Color;
pub use error::{DomainError, DomainResult};
