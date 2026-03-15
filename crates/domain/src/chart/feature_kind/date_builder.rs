//! Builder pattern for creating date/time field configurations with chainable methods.
use super::{ChartID, FeatureKind};
use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use specta::Type;
use typed_builder::TypedBuilder;

/// Precision at which a date/time value should be captured and displayed.
///
/// This is a schema-level concern, not a UI-only one. A "Birthday" feature
/// wants `Day`; a "Meeting start" feature wants `Minute`. The stored value
/// is always a `NaiveDateTime`, but sub-granularity fields are zeroed on
/// write and hidden from the editor so users can't enter meaningless
/// precision.
///
/// The variant names mirror `@internationalized/date` and Nuxt UI's
/// `UInputDate` `granularity` prop so the frontend can map 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum DateGranularity {
    /// Date only — time component is zeroed (00:00:00).
    Day,
    /// Date + hour — minutes and seconds zeroed.
    Hour,
    /// Date + hour + minute — seconds zeroed.
    Minute,
    /// Full precision.
    Second,
}

impl Default for DateGranularity {
    fn default() -> Self {
        DateGranularity::Day
    }
}

/// Builder for date/time features.
#[derive(Debug, Clone, TypedBuilder, PartialEq, Serialize, Deserialize)]
#[builder(field_defaults(default, setter(strip_option, prefix = "with_")))]
pub struct Date {
    min_date: Option<NaiveDate>,
    min_time: Option<NaiveTime>,
    max_date: Option<NaiveDate>,
    max_time: Option<NaiveTime>,
    timezone: Option<String>,
    #[builder(setter(!strip_option))]
    granularity: DateGranularity,
}

impl From<Date> for FeatureKind {
    fn from(d: Date) -> Self {
        FeatureKind::Date {
            min_date: d.min_date,
            min_time: d.min_time,
            max_date: d.max_date,
            max_time: d.max_time,
            timezone: d.timezone,
            granularity: d.granularity,
        }
    }
}

impl Default for Date {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_builder_defaults() {
        let reference = Date::default();

        if let FeatureKind::Date {
            min_date,
            min_time,
            max_date,
            max_time,
            timezone,
            granularity,
        } = reference.into()
        {
            assert_eq!(min_date, None);
            assert_eq!(min_time, None);
            assert_eq!(max_date, None);
            assert_eq!(max_time, None);
            assert_eq!(timezone, None);
            assert_eq!(granularity, DateGranularity::Day);
        } else {
            panic!("Expected Date variant");
        }
    }
}
