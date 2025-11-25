//! Rich references with display mode information.
//!
//! This module provides RichReference, which combines a BlockID with display mode
//! information to control how references are rendered in the UI.
use super::MarkID;
use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};

/// Display mode for rendering references in the UI.
/// Determines how referenced Landmarks should be visually presented.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[frb(non_opaque, unignore)]
#[serde(rename_all = "kebab-case")]
pub enum ReferenceDisplayMode {
    /// Small circular avatar (typically for people)
    Avatar,
    /// Inline mention with @ symbol (e.g., @username)
    Mention,
    /// Displays the Reference's value within the currently visible block
    Inline,
    /// Rich embedded preview
    Embed,
    /// Extra small card (icon + title only)
    CardXS,
    /// Small card (icon + title + brief metadata)
    CardSM,
    /// Medium card (icon + title + description)
    CardMD,
    /// Large card (icon + title + description + metadata)
    CardLG,
    /// Extra large card (full preview with rich content)
    CardXL,
}

impl Default for ReferenceDisplayMode {
    #[frb(ignore)]
    fn default() -> Self {
        ReferenceDisplayMode::Inline
    }
}

/// A reference that includes display mode information.
/// Used for top-level Thing fields like children, created_by, updated_by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[frb(non_opaque, unignore)]
pub struct RichReference {
    /// The referenced block ID
    pub id: MarkID,
    /// How to display this reference in the UI
    #[serde(rename = "as", default)]
    pub display_mode: ReferenceDisplayMode,
}

impl RichReference {
    /// Create a new RichReference from the given ID.
    #[frb(sync)]
    pub fn from_id(id: MarkID) -> Self {
        Self::new(id, ReferenceDisplayMode::default())
    }

    /// Create a new RichReference with the given ID and display mode.
    #[frb(sync)]
    pub fn new(id: MarkID, mode: ReferenceDisplayMode) -> Self {
        Self {
            id,
            display_mode: mode,
        }
    }
}

/// Convert from BlockID to RichReference with default display mode
impl From<MarkID> for RichReference {
    fn from(id: MarkID) -> Self {
        Self::from_id(id)
    }
}

/// Extract BlockID from RichReference
impl From<RichReference> for MarkID {
    fn from(rich_ref: RichReference) -> Self {
        rich_ref.id
    }
}
