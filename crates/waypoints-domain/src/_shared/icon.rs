//! Flexible Icon system with hierarchical prefixes and dynamic colorability detection.
//!
//! This module provides a lightweight Icon system that uses hierarchical identifier
//! strings to represent different types of icons. The colorability is determined
//! dynamically via the Iconify API when possible.
//!
//! # Icon Identifier Format
//!
//! Icons use a hierarchical prefix system:
//! - `iconify:<set>:<name>` - Iconify icons (e.g., `iconify:mdi:home`)
//! - `waypoints:<set>:<name>` - Custom Waypoints icons (future)
//! - `system:emoji:<name>` - Platform emoji (e.g., `system:emoji:house`)
//! - `local-image:<path>` - Local asset images (e.g., `local-image:assets/logo.png`)
//! - `remote-image:<url>` - Remote images (e.g., `remote-image:https://...`)
//!
//! # Examples
//!
//! ```
//! use waypoints_domain::Icon;
//!
//! // Iconify icons - colorability determined by API
//! let home = Icon::iconify("mdi", "home");
//! assert_eq!(home.identifier, "iconify:mdi:home");
//!
//! let settings = Icon::iconify("heroicons-outline", "cog-6-tooth");
//! assert_eq!(settings.identifier, "iconify:heroicons-outline:cog-6-tooth");
//!
//! // Non-colorable icons
//! let emoji = Icon::system_emoji("house");
//! assert_eq!(emoji.identifier, "system:emoji:house");
//!
//! let image = Icon::local_image("assets/logo.png");
//! assert_eq!(image.identifier, "local-image:assets/logo.png");
//!
//! let remote = Icon::remote_image("https://example.com/icon.svg");
//! assert_eq!(remote.identifier, "remote-image:https://example.com/icon.svg");
//!
//! // Parse from strings using FromStr
//! let parsed: Icon = "iconify:mdi:home".parse().unwrap();
//! assert_eq!(parsed.identifier, "iconify:mdi:home");
//!
//! // Check colorability (may require API call)
//! // This would be handled by the UI layer in practice
//! assert_eq!(emoji.is_colorable(), Some(false));
//! ```

use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::{DomainError, DomainResult};

/// A flexible icon representation using hierarchical identifiers.
///
/// Icons are represented by structured identifier strings that indicate the source,
/// type, and specific icon. The UI layer is responsible for resolving these identifiers
/// to actual rendered icons and determining colorability via appropriate APIs.
/// flutter_rust_bridge:non_opaque
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[frb(non_opaque, unignore)]
pub struct Icon {
    /// Hierarchical icon identifier (e.g., "iconify:mdi:home", "system:emoji:house")
    pub identifier: String,

    /// Alternative text for accessibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,

    /// Cached colorability information (determined by UI layer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colorable: Option<bool>,

    /// Additional metadata for UI layer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Icon namespace for different icon sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[frb(non_opaque, unignore)]
pub enum IconNamespace {
    /// Iconify icon collection
    Iconify { set: String, name: String },

    /// Waypoints custom icons (future)
    Waypoints { set: String, name: String },

    /// System/platform emoji
    SystemEmoji { name: String },

    /// Local image file or asset
    LocalImage { path: String },

    /// Remote image URL
    RemoteImage { url: String },
}

impl Icon {
    /// Create a new icon with the given identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use waypoints_domain::Icon;
    ///
    /// let icon = Icon::new("iconify:mdi:home");
    /// assert_eq!(icon.identifier, "iconify:mdi:home");
    /// ```
    #[frb(sync)]
    pub fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            alt_text: None,
            colorable: None,
            metadata: None,
        }
    }

    /// Create an Iconify icon with hierarchical identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use waypoints_domain::Icon;
    ///
    /// let home = Icon::iconify("mdi", "home");
    /// assert_eq!(home.identifier, "iconify:mdi:home");
    ///
    /// let settings = Icon::iconify("heroicons-outline", "cog-6-tooth");
    /// assert_eq!(settings.identifier, "iconify:heroicons-outline:cog-6-tooth");
    /// ```
    #[frb(sync)]
    pub fn iconify(set: &str, name: &str) -> Self {
        Self {
            alt_text: format!("Icon: {set}:{name}").into(),
            ..Self::new(&format!("iconify:{set}:{name}"))
        }
    }

    /// Create a Waypoints custom icon (future extensibility).
    ///
    /// # Examples
    ///
    /// ```
    /// use waypoints_domain::Icon;
    ///
    /// let custom = Icon::waypoints("ui", "special-button");
    /// assert_eq!(custom.identifier, "waypoints:ui:special-button");
    /// ```
    #[frb(sync)]
    pub fn waypoints(set: &str, name: &str) -> Self {
        Self {
            alt_text: format!("Waypoints icon: {set}:{name}").into(),
            colorable: false.into(),
            ..Self::new(&format!("waypoints:{set}:{name}"))
        }
    }

    /// Create a system emoji icon.
    ///
    /// # Examples
    ///
    /// ```
    /// use waypoints_domain::Icon;
    ///
    /// let house = Icon::system_emoji("house");
    /// assert_eq!(house.identifier, "system:emoji:house");
    ///
    /// // Can also use actual emoji characters
    /// let emoji = Icon::system_emoji("🏠");
    /// assert_eq!(emoji.identifier, "system:emoji:🏠");
    /// ```
    #[frb(sync)]
    pub fn system_emoji(name: &str) -> Self {
        Self {
            alt_text: format!("Emoji: {name}").into(),
            colorable: false.into(),
            ..Self::new(&format!("system:emoji:{name}"))
        }
    }

    /// Create a local image icon.
    ///
    /// # Examples
    ///
    /// ```
    /// use waypoints_domain::Icon;
    ///
    /// let logo = Icon::local_image("assets/logo.png");
    /// assert_eq!(logo.identifier, "local-image:assets/logo.png");
    /// ```
    #[frb(sync)]
    pub fn local_image(path: &str) -> Self {
        Self {
            alt_text: format!("Image: {}", path.split('/').last().unwrap_or(path)).into(),
            colorable: false.into(),
            ..Self::new(&format!("local-image:{}", path))
        }
    }

    /// Create a remote image icon.
    ///
    /// # Examples
    ///
    /// ```
    /// use waypoints_domain::Icon;
    ///
    /// let remote = Icon::remote_image("https://example.com/icon.svg");
    /// assert_eq!(remote.identifier, "remote-image:https://example.com/icon.svg");
    /// ```
    #[frb(sync)]
    pub fn remote_image(url: &str) -> Self {
        Self {
            alt_text: "Remote image".to_string().into(),
            colorable: false.into(),
            ..Self::new(&format!("remote-image:{}", url))
        }
    }

    /// Parse the icon namespace from the identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use waypoints_domain::{Icon, IconNamespace};
    ///
    /// let icon = Icon::iconify("mdi", "home");
    /// match icon.parse_namespace() {
    ///     Some(IconNamespace::Iconify { set, name }) => {
    ///         assert_eq!(set, "mdi");
    ///         assert_eq!(name, "home");
    ///     }
    ///     _ => panic!("Expected Iconify namespace"),
    /// }
    /// ```
    pub fn parse_namespace(&self) -> Option<IconNamespace> {
        use IconNamespace::*;
        let parts: Vec<&str> = self.identifier.splitn(3, ':').collect();

        match parts.as_slice() {
            ["iconify", set, name] => Some(Iconify {
                set: set.to_string(),
                name: name.to_string(),
            }),
            ["waypoints", set, name] => Some(Waypoints {
                set: set.to_string(),
                name: name.to_string(),
            }),
            ["system", "emoji", name] => Some(SystemEmoji {
                name: name.to_string(),
            }),
            ["local-image", path] => Some(LocalImage {
                path: path.to_string(),
            }),
            ["remote-image", url] => Some(RemoteImage {
                url: url.to_string(),
            }),
            _ => None,
        }
    }

    /// Check if this icon is colorable.
    ///
    /// For Iconify icons, this should be determined by querying the Iconify API.
    /// For other types, it's determined based on the icon type.
    ///
    /// Returns None if colorability hasn't been determined yet (requires API call).
    #[frb(sync)]
    pub fn is_colorable(&self) -> Option<bool> {
        // If we have cached colorability info, return it
        if let Some(colorable) = self.colorable {
            return Some(colorable);
        }

        // For non-Iconify types, we can determine colorability statically
        match self.parse_namespace()? {
            IconNamespace::Iconify { .. } => None,   // Requires API call
            IconNamespace::Waypoints { .. } => None, // Would require API call
            IconNamespace::SystemEmoji { .. } => Some(false),
            IconNamespace::LocalImage { .. } => Some(false),
            IconNamespace::RemoteImage { .. } => Some(false),
        }
    }
}

impl FromStr for Icon {
    type Err = DomainError;

    /// Parse an icon from a string, trying to intelligently determine the type.
    ///
    /// This method provides backward compatibility with existing string-based icons.
    ///
    /// # Examples
    ///
    /// ```
    /// use waypoints_domain::Icon;
    /// use std::str::FromStr;
    ///
    /// // Already hierarchical - pass through
    /// let hierarchical = Icon::from_str("iconify:mdi:home").unwrap();
    /// assert_eq!(hierarchical.identifier, "iconify:mdi:home");
    ///
    /// // Can also use .parse()
    /// let legacy: Icon = "iconify:mdi:home".parse().unwrap();
    /// assert_eq!(legacy.identifier, "iconify:mdi:home");
    ///
    /// // URLs
    /// let url_icon: Icon = "https://example.com/icon.png".parse().unwrap();
    /// assert_eq!(url_icon.identifier, "remote-image:https://example.com/icon.png");
    ///
    /// // Plain emoji
    /// let emoji: Icon = "🏠".parse().unwrap();
    /// assert_eq!(emoji.identifier, "system:emoji:🏠");
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(DomainError::ParseIconFailed("".into()));
        }

        // Check if it's already in hierarchical format
        if s.starts_with("iconify:")
            || s.starts_with("waypoints:")
            || s.starts_with("system:")
            || s.starts_with("local-image:")
            || s.starts_with("remote-image:")
        {
            return Ok(Icon::new(s));
        }

        // Handle URLs
        if s.starts_with("http://") || s.starts_with("https://") {
            return Ok(Icon::remote_image(s));
        }

        // Handle file paths (contain / or \)
        if s.contains('/') || s.contains('\\') {
            return Ok(Icon::local_image(s));
        }

        // Handle single emoji characters or short unicode strings
        if s.chars().count() <= 4 && s.chars().any(|c| c as u32 > 127) {
            return Ok(Icon::system_emoji(s));
        }

        Err(DomainError::ParseIconFailed(s.into()))
    }
}

impl fmt::Display for Icon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl Default for Icon {
    fn default() -> Self {
        Self::system_emoji("📄") // Default page icon
    }
}

/// Trait for components that can have icons.
pub trait HasIcon {
    /// Get the icon if present.
    fn get_icon(&self) -> Option<Icon>;

    /// Set the icon.
    fn set_icon(&mut self, icon: Option<Icon>);

    /// Check if this component has a colorable icon.
    /// Returns None if colorability hasn't been determined yet.
    fn has_colorable_icon(&self) -> Option<bool> {
        self.get_icon().and_then(|icon| icon.is_colorable())
    }
}

/// Common icon presets for Waypoints components.
pub mod presets {
    use super::Icon;

    /// Default icons for chart types
    pub mod charts {
        use super::Icon;

        pub fn page() -> Icon {
            Icon::iconify("heroicons", "document-solid")
        }
        pub fn objective() -> Icon {
            Icon::iconify("heroicons", "check-circle-solid")
        }
        pub fn charter() -> Icon {
            Icon::iconify("heroicons", "rocket-launch-solid")
        }
        pub fn note() -> Icon {
            Icon::iconify("heroicons", "document-text-solid")
        }
        pub fn person() -> Icon {
            Icon::iconify("heroicons", "user-solid")
        }
        pub fn meeting() -> Icon {
            Icon::iconify("heroicons", "user-group-solid")
        }
        pub fn event() -> Icon {
            Icon::iconify("heroicons", "calendar-solid")
        }
        pub fn bookmark() -> Icon {
            Icon::iconify("heroicons", "bookmark-solid")
        }
        pub fn journal() -> Icon {
            Icon::iconify("heroicons", "puzzle-piece-solid")
        }

        /// Get default icon for a chart type by name
        pub fn for_chart_name(name: &str) -> Option<Icon> {
            match name.to_lowercase().as_str() {
                "page" => Some(page()),
                "task" => Some(objective()),
                "project" => Some(charter()),
                "note" => Some(note()),
                "person" => Some(person()),
                "meeting" => Some(meeting()),
                "event" => Some(event()),
                _ => None,
            }
        }
    }

    /// Common action icons
    pub mod actions {
        use super::Icon;

        pub fn add() -> Icon {
            Icon::iconify("heroicons", "plus-solid")
        }
        pub fn remove() -> Icon {
            Icon::iconify("heroicons", "minus-solid")
        }
        pub fn delete() -> Icon {
            Icon::iconify("heroicons", "trash-solid")
        }
        pub fn edit() -> Icon {
            Icon::iconify("heroicons", "pencil-solid")
        }
        pub fn save() -> Icon {
            Icon::iconify("heroicons", "check-solid")
        }
        pub fn cancel() -> Icon {
            Icon::iconify("heroicons", "x-mark-solid")
        }
        pub fn settings() -> Icon {
            Icon::iconify("heroicons", "cog-6-tooth-solid")
        }
        pub fn search() -> Icon {
            Icon::iconify("heroicons", "magnifying-glass-solid")
        }
        pub fn filter() -> Icon {
            Icon::iconify("heroicons", "funnel-solid")
        }
        pub fn sort() -> Icon {
            Icon::iconify("heroicons", "bars-arrow-up-solid")
        }
        pub fn menu() -> Icon {
            Icon::iconify("heroicons", "bars-3-solid")
        }
        pub fn close() -> Icon {
            Icon::iconify("heroicons", "x-mark-solid")
        }
    }

    /// Status and feedback icons
    pub mod status {
        use super::Icon;

        pub fn success() -> Icon {
            Icon::iconify("heroicons", "check-circle-solid")
        }
        pub fn warning() -> Icon {
            Icon::iconify("heroicons", "exclamation-triangle-solid")
        }
        pub fn error() -> Icon {
            Icon::iconify("heroicons", "x-circle-solid")
        }
        pub fn info() -> Icon {
            Icon::iconify("heroicons", "information-circle-solid")
        }
        pub fn loading() -> Icon {
            Icon::iconify("heroicons", "arrow-path-solid")
        }
        pub fn pending() -> Icon {
            Icon::iconify("heroicons", "clock-solid")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_icon_creation() {
        let iconify = Icon::iconify("mdi", "home");
        assert_eq!(iconify.identifier, "iconify:mdi:home");

        let waypoints = Icon::waypoints("ui", "special");
        assert_eq!(waypoints.identifier, "waypoints:ui:special");

        let emoji = Icon::system_emoji("house");
        assert_eq!(emoji.identifier, "system:emoji:house");

        let local = Icon::local_image("assets/logo.png");
        assert_eq!(local.identifier, "local-image:assets/logo.png");

        let remote = Icon::remote_image("https://example.com/icon.svg");
        assert_eq!(
            remote.identifier,
            "remote-image:https://example.com/icon.svg"
        );
    }

    #[test]
    fn test_namespace_parsing() {
        let icon = Icon::iconify("mdi", "home");
        match icon.parse_namespace().unwrap() {
            IconNamespace::Iconify { set, name } => {
                assert_eq!(set, "mdi");
                assert_eq!(name, "home");
            }
            _ => panic!("Expected Iconify namespace"),
        }

        let emoji = Icon::system_emoji("house");
        match emoji.parse_namespace().unwrap() {
            IconNamespace::SystemEmoji { name } => {
                assert_eq!(name, "house");
            }
            _ => panic!("Expected SystemEmoji namespace"),
        }
    }

    #[test]
    fn test_colorability_detection() {
        // Iconify icons require API call - should return None without cache
        let iconify = Icon::iconify("mdi", "home");
        assert_eq!(iconify.is_colorable(), None);

        // With cached info
        let iconify_cached = Icon {
            colorable: true.into(),
            ..Icon::iconify("mdi", "home")
        };
        assert_eq!(iconify_cached.is_colorable(), Some(true));

        // Non-colorable types should return Some(false)
        let emoji = Icon::system_emoji("house");
        assert_eq!(emoji.is_colorable(), Some(false));

        let image = Icon::local_image("logo.png");
        assert_eq!(image.is_colorable(), Some(false));
    }

    #[test]
    fn test_presets() {
        let page = presets::charts::page();
        assert_eq!(page.identifier, "iconify:heroicons:document-solid");

        let add = presets::actions::add();
        assert_eq!(add.identifier, "iconify:heroicons:plus-solid");

        // Test chart name lookup
        let task_icon = presets::charts::for_chart_name("task").unwrap();
        assert_eq!(task_icon.identifier, "iconify:heroicons:check-circle-solid");

        let unknown = presets::charts::for_chart_name("unknown");
        assert!(unknown.is_none());
    }
}
