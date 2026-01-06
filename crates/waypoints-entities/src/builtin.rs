//! Built-in Thing types that provide common schemas out of the box.
//!
//! These built-ins serve as:
//! - Ready-to-use types for common use cases
//! - Templates that users can extend or customize
//! - Defaults that can be reset if users break their schemas
//!
//! The built-in system follows these principles:
//! - Each built-in has a fixed TypeID for consistency
//! - Users can extend built-ins by copying and modifying them
//! - Users can reset to original built-in schemas at any time
//! - Built-ins cover common productivity and content management needs
//!
//! # Examples
//!
//! ## Using Built-in Charts
//
// TODO add examples of working with built-in Charts
//!
//! ## Working with Different Built-in Types
//!
// TODO add examples of working with the different Chart types
//!

use std::collections::HashMap;
use waypoints_domain::chart::{Chart, ChartID};

// Import built-in type modules
mod _utils;
pub mod page;

use _utils::tags_feature;

/// Registry of all built-in [`Landmark`] types.
/// This provides a centralized way to access, extend, and reset built-in [Charts][`Chart`].
///
/// # Examples
///
/// ## Basic Usage
///
// TODO add basic usage examples
///
/// ## Extending Built-ins
///
// TODO add examples that extend built-ins
///
#[derive(Debug, Clone)]
pub struct BuiltinRegistry {
    charts: HashMap<ChartID, Chart>,
}

impl BuiltinRegistry {
    /// Create a new registry with all built-in types loaded.
    pub fn new() -> Self {
        let mut registry = Self {
            charts: HashMap::new(),
        };

        // Load all built-in types from their individual modules
        for chart in builtin_charts() {
            registry.register(chart);
        }

        registry
    }

    /// Register a built-in type.
    fn register(&mut self, chart: Chart) {
        self.charts.insert(chart.id.clone(), chart);
    }

    /// Get a built-in type by ID.
    pub fn get(&self, type_id: ChartID) -> Option<&Chart> {
        self.charts.get(&type_id)
    }

    /// Get all built-in types.
    pub fn all(&self) -> Vec<&Chart> {
        self.charts.values().collect()
    }

    /// Get all built-in type IDs.
    pub fn type_ids(&self) -> Vec<ChartID> {
        self.charts.keys().cloned().collect()
    }

    /// Check if a type ID is a built-in.
    pub fn is_builtin(&self, type_id: ChartID) -> bool {
        self.charts.contains_key(&type_id)
    }

    /// Create a customizable copy of a built-in type.
    /// This allows users to extend built-ins while preserving the originals.
    pub fn extend_builtin(&self, builtin_id: ChartID, new_name: String) -> Option<Chart> {
        self.get(builtin_id).map(|builtin| {
            let mut extended = builtin.clone();
            extended.id = ChartID::new(); // New ID for the extended type
            extended.name = new_name.clone();
            extended.description = Some(format!("Extended from built-in {}", builtin.name));
            extended
        })
    }

    /// Reset a user type back to its built-in version.
    /// Useful when users have modified a built-in and want to start over.
    pub fn reset_to_builtin(&self, builtin_id: ChartID) -> Option<Chart> {
        self.get(builtin_id).cloned()
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to get all built-in [Charts][`Chart`] as a vector.
/// Useful for listing available types without creating a full registry.
///
/// # Examples
///
/// ```
/// use waypoints_entities::builtin::builtin_charts;
///
/// let charts = builtin_charts();
///
/// // All should be builtin
/// for chart in &charts {
///     assert!(chart.is_builtin(), "{} should be builtin", chart.name);
/// }
///
/// // Should include common types
/// let chart_names: Vec<&str> = charts.iter().map(|c| c.name.as_str()).collect();
/// assert!(chart_names.contains(&"Page"));
/// ```
pub fn builtin_charts() -> Vec<Chart> {
    vec![
        page::chart(),
        // Additional builtin charts will be added incrementally:
        // - Note, Person, Objective, Meeting, Campaign, Bookmark, Journal
    ]
}

/// Convenience function to get all built-in type IDs.
pub fn builtin_chart_ids() -> Vec<ChartID> {
    builtin_charts().into_iter().map(|chart| chart.id).collect()
}
