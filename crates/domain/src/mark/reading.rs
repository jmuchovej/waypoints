use super::{MarkID, RichText, reading_validators as validators};
use crate::{Feature, FeatureKind, GlyphID};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

/// Instances of [`Features`][F] with their relevant data on a [`Landmark`][M:L].
///
/// [F]: crate::chart::Feature
/// [M:L]: crate::mark::Landmark
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum Reading {
    Text(RichText),
    Number(f64),
    Bool(bool),
    /// Single date/datetime
    Date(NaiveDateTime),
    /// Date range with optional start/end dates and times
    DateRange(NaiveDateTime, NaiveDateTime),
    Select(GlyphID),
    MultiSelect(Vec<GlyphID>),
    GroupedSelect(HashMap<GlyphID, Vec<GlyphID>>),
    References(Vec<MarkID>),
    #[serde(rename = "url")]
    URL(String),
    Email(String),
    Phone(String, String),
    File(String),
    Formula(String),
    Null,
}

impl FeatureKind {
    /// Validate a reading against this feature's constraints using enum dispatch
    pub fn validate_reading(&self, reading: &Reading) -> Result<(), String> {
        match self {
            FeatureKind::Text => validators::text(reading, self),
            FeatureKind::Number { .. } => validators::number(reading, self),
            FeatureKind::Checkbox => validators::checkbox(reading, self),
            FeatureKind::Date { .. } => validators::date(reading, self),
            FeatureKind::Email => validators::email(reading, self),
            FeatureKind::Phone => validators::phone(reading, self),
            FeatureKind::URL => validators::url(reading, self),
            FeatureKind::Select { .. } => validators::select(reading, self),
            FeatureKind::MultiSelect { .. } => validators::multi_select(reading, self),
            FeatureKind::GroupedSelect { .. } => validators::grouped_select(reading, self),
            FeatureKind::GroupedMultiSelect { .. } => {
                validators::grouped_multi_select(reading, self)
            }
            FeatureKind::Reference { .. } => validators::reference(reading, self),
            FeatureKind::Formula => validators::formula(reading, self),
        }
    }
}
