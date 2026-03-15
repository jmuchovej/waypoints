//! Shared utilities and types for the waypoints domain.

pub type Json = serde_json::Value;
pub type DateTime = chrono::NaiveDateTime;
pub type Uuid = uuid::Uuid;

pub mod color;
pub mod error;
pub mod fractional_index;
pub mod icon;

pub use color::Color;
pub use error::{DomainError, DomainResult};
pub use icon::{HasIcon, Icon, IconNamespace, presets as icon_presets};
