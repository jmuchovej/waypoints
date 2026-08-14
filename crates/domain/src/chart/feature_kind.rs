//! Feature implementations for chart validation.
//!
//! Each feature type handles its own validation logic, default values,
//! and JSON conversion. Features define the structure of data fields
//! while validation logic is handled in feature_validators.
use super::{ChartID, FeatureID, GlyphID};
use crate::{Color, DomainResult, Icon, Json, mark::ReferenceDisplayMode};
use chrono::{NaiveDate, NaiveTime};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// Field types that can be used in Charts.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum FeatureKind {
    #[default]
    Text,

    /// Numeric field with range constraints
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
        precision: Option<u8>,
        format: Option<String>,
    },

    /// Boolean checkbox field
    Checkbox,

    /// Date and time values with optional ranges
    Date {
        min_date: Option<NaiveDate>,
        min_time: Option<NaiveTime>,
        max_date: Option<NaiveDate>,
        max_time: Option<NaiveTime>,
        timezone: Option<String>,
        #[serde(default)]
        granularity: DateGranularity,
    },

    /// Email field with basic validation
    Email,

    /// Phone number field with basic validation
    Phone,

    /// URL field with basic validation
    #[serde(rename = "url")]
    URL,

    Select {
        options: IndexMap<GlyphID, SelectOption>,
        default_option: Option<GlyphID>,
    },

    /// Multiple selection from options
    MultiSelect {
        options: IndexMap<GlyphID, SelectOption>,
        max_selections: Option<u32>,
        #[serde(default)]
        default_options: Vec<GlyphID>,
    },

    /// Grouped single selection (state machine style)
    GroupedSelect {
        groups: IndexMap<GlyphID, SelectGroup>,
        default_option: Option<GlyphID>,
    },

    /// Grouped multiple selection
    GroupedMultiSelect {
        groups: IndexMap<GlyphID, SelectGroup>,
        max_selections: Option<u32>,
        default_options: IndexMap<GlyphID, GlyphID>,
    },

    /// Reference to other Things
    Reference {
        allowed_types: Option<Vec<ChartID>>,
        max_references: Option<u32>,
        #[serde(default = "default_display_field")]
        display_field: String,
        #[serde(rename = "as", default)]
        display_mode: ReferenceDisplayMode,
        #[specta(type = Option<specta_typescript::Unknown>)]
        filter_criteria: Option<Json>,
    },

    Formula,
}

fn default_display_field() -> String {
    "title".to_string()
}

pub mod date_builder;
pub mod number_builder;
pub mod reference_builder;
pub mod select_builder;

pub use date_builder::{Date, DateGranularity};
pub use number_builder::Number;
pub use reference_builder::Reference;
pub use select_builder::{Select, SelectGroup, SelectOption};
