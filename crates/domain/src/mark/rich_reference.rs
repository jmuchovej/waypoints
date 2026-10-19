//! Rich references with display mode information.
//!
//! This module provides RichReference, which combines a BlockID with display mode
//! information to control how references are rendered in the UI.
use super::MarkID;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Display mode for rendering references in the UI.
/// Determines how referenced Landmarks should be visually presented.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum ReferenceDisplayMode {
    /// Small circular avatar (typically for people)
    Avatar,
    /// Inline mention with @ symbol (e.g., @username)
    Mention,
    /// Displays the Reference's value within the currently visible block
    #[default]
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

/// A reference that includes display mode information.
///
/// `RichReference` is the typed payload for [`InlineMark`][I] references
/// within [`MarkData::Content`][C] (e.g., mentions, inline cards, embeds).
///
/// Deliberately not re-exported from the crate root nor registered for
/// TypeScript: nothing consumes it yet.
///
/// [I]: super::InlineMark
/// [C]: super::MarkData::Content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct RichReference {
    /// The referenced block ID
    pub id: MarkID,
    /// How to display this reference in the UI
    #[serde(rename = "as", default)]
    pub display_mode: ReferenceDisplayMode,
}

impl RichReference {
    /// Create a new RichReference from the given ID.
    pub fn from_id(id: MarkID) -> Self {
        Self::new(id, ReferenceDisplayMode::default())
    }

    /// Create a new RichReference with the given ID and display mode.
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
