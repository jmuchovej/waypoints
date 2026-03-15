//! Builder pattern for creating number field configurations with chainable methods.
use super::{ChartID, FeatureKind};
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// Builder pattern for Numeric fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypedBuilder)]
#[builder(
    field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")),
    build_method(into)
)]
pub struct Number {
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    precision: Option<u8>,
    format: Option<String>,
}

impl From<Number> for FeatureKind {
    fn from(n: Number) -> Self {
        FeatureKind::Number {
            min: n.min,
            max: n.max,
            step: n.step,
            precision: n.precision,
            format: n.format,
        }
    }
}

impl Default for Number {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_builder_defaults() {
        let reference = Number::default().into();

        if let FeatureKind::Number {
            min,
            max,
            step,
            precision,
            format,
        } = reference
        {
            assert_eq!(min, None);
            assert_eq!(max, None);
            assert_eq!(step, None);
            assert_eq!(precision, None);
            assert_eq!(format, None);
        } else {
            panic!("Expected Reference variant");
        }
    }

    #[test]
    fn test_number_builder_min_max() {
        let reference = Number::builder().with_min(10.0).with_max(20.0).build();

        if let FeatureKind::Number { min, max, .. } = reference {
            assert_eq!(min.unwrap(), 10.0);
            assert_eq!(max.unwrap(), 20.0);
        } else {
            panic!("Expected Reference variant");
        }
    }

    #[test]
    fn test_number_builder_step_precision() {
        let reference = Number::builder().with_step(1.0).with_precision(5).build();

        if let FeatureKind::Number {
            step, precision, ..
        } = reference
        {
            assert_eq!(step.unwrap(), 1.0);
            assert_eq!(precision.unwrap(), 5);
        } else {
            panic!("Expected Reference variant");
        }
    }
}
