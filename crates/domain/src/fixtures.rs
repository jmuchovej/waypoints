//! Terse builders for tests in the crates that know the domain.
//!
//! Nothing here validates: fixtures use ids the parsers would refuse.

use chrono::{NaiveDate, NaiveDateTime};
use indexmap::IndexMap;
use uuid::Uuid;

use crate::{Base36ID, FeatureID, FeatureKind, GlyphID, MarkID, ReferenceDisplayMode};

/// A [`FeatureID`], spelled as given.
pub fn feature_id(id: &str) -> FeatureID {
    FeatureID(Base36ID(id.to_string()))
}

/// A [`GlyphID`], spelled as given.
pub fn glyph_id(id: &str) -> GlyphID {
    GlyphID(Base36ID(id.to_string()))
}

/// A [`MarkID`] with a recognisable value.
pub fn mark_id(n: u128) -> MarkID {
    MarkID(Uuid::from_u128(n))
}

/// An instant `milli` milliseconds past a fixed minute.
pub fn at(milli: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 2)
        .unwrap()
        .and_hms_milli_opt(9, 41, 0, milli)
        .unwrap()
}

/// [`FeatureKind::Number`] with no constraints.
pub fn number_kind() -> FeatureKind {
    FeatureKind::Number {
        min: None,
        max: None,
        step: None,
        precision: None,
        format: None,
    }
}

/// [`FeatureKind::Date`] with no constraints.
pub fn date_kind() -> FeatureKind {
    FeatureKind::Date {
        min_date: None,
        min_time: None,
        max_date: None,
        max_time: None,
        timezone: None,
        granularity: Default::default(),
    }
}

/// [`FeatureKind::Reference`] with no constraints.
pub fn reference_kind() -> FeatureKind {
    FeatureKind::Reference {
        allowed_types: None,
        max_references: None,
        display_field: "title".to_string(),
        display_mode: ReferenceDisplayMode::default(),
        filter_criteria: None,
    }
}

/// [`FeatureKind::Select`] with no options.
pub fn select_kind() -> FeatureKind {
    FeatureKind::Select {
        options: IndexMap::new(),
        default_option: None,
    }
}
