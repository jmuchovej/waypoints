//! Builder pattern for creating number field configurations with chainable methods.
use super::{ChartID, FeatureKind};
use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// Builder pattern for Numeric fields
#[derive(Debug, Clone, TypedBuilder, PartialEq, Serialize, Deserialize)]
#[builder(field_defaults(default, setter(strip_option, prefix = "with_")))]
pub struct Date {
    min_date: Option<NaiveDate>,
    min_time: Option<NaiveTime>,
    max_date: Option<NaiveDate>,
    max_time: Option<NaiveTime>,
    timezone: Option<String>,
}

impl From<Date> for FeatureKind {
    fn from(d: Date) -> Self {
        FeatureKind::Date {
            min_date: d.min_date,
            min_time: d.min_time,
            max_date: d.max_date,
            max_time: d.max_time,
            timezone: d.timezone,
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
        } = reference.into()
        {
            assert_eq!(min_date, None);
            assert_eq!(min_time, None);
            assert_eq!(max_date, None);
            assert_eq!(max_time, None);
            assert_eq!(timezone, None);
        } else {
            panic!("Expected Reference variant");
        }
    }
}
