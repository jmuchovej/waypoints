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
//!
//! ```
//! use waypoints::builtins;
//! use waypoints::landmark::Landmark;
//! use waypoints::mark::MarkID;
//! use serde_json::json;
//!
//! let creator_id = MarkID::new();
//!
//! // Create a task using the built-in task chart
//! let task_chart = builtins::task::task_chart();
//! let mut task = Landmark::new(task_chart.id.clone(), creator_id);
//! task.title = "Learn Rust".to_string();
//!
//! // Set task-specific fields
//! let completed_feature = task_chart.features.iter()
//!     .find(|f| f.title == "Completed")
//!     .unwrap();
//! task.set_field(completed_feature.id.clone(), json!(false));
//!
//! assert_eq!(task.title, "Learn Rust");
//! assert_eq!(task.get_field(&completed_feature.id), Some(&json!(false)));
//! ```
//!
//! ## Working with Different Built-in Types
//!
//! ```
//! use waypoints::builtins;
//! use waypoints::landmark::Landmark;
//! use waypoints::mark::MarkID;
//!
//! let creator_id = MarkID::new();
//!
//! // Create different types of landmarks
//! let page = Landmark::new(builtins::page::page_chart().id, creator_id);
//! let note = Landmark::new(builtins::note::note_chart().id, creator_id);
//! let person = Landmark::new(builtins::person::person_chart().id, creator_id);
//!
//! // Each has different available features
//! let page_chart = builtins::page::page_chart();
//! let note_chart = builtins::note::note_chart();
//! let person_chart = builtins::person::person_chart();
//!
//! assert_eq!(page_chart.features.len(), 0); // Page has no custom features
//! assert!(note_chart.features.len() > 0); // Note has tags, source URL, etc.
//! assert!(person_chart.features.len() > 0); // Person has email, phone, etc.
//! ```

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
/// ```
/// use waypoints::builtins::BuiltinRegistry;
/// use waypoints::builtins;
///
/// let registry = BuiltinRegistry::new();
///
/// // Get a built-in chart
/// let task_id = builtins::task::task_type_id();
/// let task_chart = registry.get(task_id).unwrap();
/// assert_eq!(task_chart.name, "Task");
/// assert!(task_chart.is_builtin());
///
/// // Check if a type is built-in
/// assert!(registry.is_builtin(builtins::page::page_chart().id));
///
/// // Get all built-in charts
/// let all_builtins = registry.all();
/// assert!(all_builtins.len() >= 9); // We have at least 9 builtin types
/// ```
///
/// ## Extending Built-ins
///
/// ```
/// use waypoints::builtins::BuiltinRegistry;
/// use waypoints::builtins;
/// use waypoints::feature::{Feature, FeatureKind};
///
/// let registry = BuiltinRegistry::new();
///
/// // Extend the task builtin with custom fields
/// let task_id = builtins::task::task_type_id();
/// let mut extended_task = registry.extend_builtin(task_id, "Enhanced Task".to_string()).unwrap();
///
/// // Add a custom priority field
/// extended_task = extended_task.add_feature(Feature::new(
///     "Difficulty".to_string(),
///     FeatureKind::unified_select_builder()
///         .with_option("Easy")
///         .with_option("Hard")
///         .build()
/// ));
///
/// assert_eq!(extended_task.name, "Enhanced Task");
/// assert!(!extended_task.is_builtin()); // Extended versions are not builtin
/// assert!(extended_task.features.len() > 5); // Has original task features + our new one
/// ```
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
/// use waypoints::builtins::builtin_charts;
///
/// let charts = builtin_charts();
/// assert!(charts.len() >= 9); // At least 9 builtin charts
///
/// // All should be builtin
/// for chart in &charts {
///     assert!(chart.is_builtin(), "{} should be builtin", chart.name);
/// }
///
/// // Should include common types
/// let chart_names: Vec<&str> = charts.iter().map(|c| c.name.as_str()).collect();
/// assert!(chart_names.contains(&"Page"));
/// assert!(chart_names.contains(&"Task"));
/// assert!(chart_names.contains(&"Note"));
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
