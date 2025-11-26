//! Builder pattern for creating reference field configurations with chainable methods.
use super::{ChartID, FeatureKind, Json, ReferenceDisplayMode};
use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

macro_rules! impl_display_mode {
    ($variant:ident => $suffix:literal) => {
        paste::paste! {
          #[doc = "Display references as " $suffix]
          fn [<with_display_ $suffix:snake >](&mut self) {
            self.display_mode = ReferenceDisplayMode::$variant;
          }
        }
    };
}

/// Builder pattern for Reference fields
#[derive(Debug, Clone, TypedBuilder, PartialEq, Serialize, Deserialize)]
#[builder(
    field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")),
    builder_type(attributes(#[frb(ignore)])),
)]
pub struct Reference {
    #[builder(
      via_mutators(init = None),
      mutators(
        /// Allow references to a single chart type
        pub fn add_allowed_chart(&mut self, chart_id: ChartID) {
          self.allowed_charts = Some(match &self.allowed_charts {
            Some(chart_ids) => [chart_ids.to_owned(), vec![chart_id]].concat(),
            None => vec![chart_id]
          });
        }

        /// Restrict references to specific chart types
        pub fn with_allowed_charts(&mut self, chart_ids: impl IntoIterator<Item = ChartID>) {
          self.allowed_charts = Some(chart_ids.into_iter().collect());
        }
      )
    )]
    allowed_charts: Option<Vec<ChartID>>,

    #[builder(
      via_mutators(init = None),
      mutators(
        /// Set a maximum number of references
        pub fn with_max_references(&mut self, max: usize) {
            self.max_references = Some(max);
        }

        /// Limit to a single reference
        pub fn with_single_reference(&mut self) {
            self.max_references = Some(1);
        }

      )
    )]
    max_references: Option<usize>,

    #[builder(
      via_mutators(init = "title".to_string()),
      mutators(
        pub fn with_display_field(&mut self, field: &str) {
          self.display_field = field.to_string();
        }
      )
    )]
    display_field: String,

    #[builder(
      via_mutators, setter(!strip_option),
      mutators(
        /// Display references as avatar
        pub fn with_display_avatar(&mut self) {
            self.display_mode = ReferenceDisplayMode::Avatar;
        }

        /// Display references inline
        pub fn with_display_inline(&mut self) {
            self.display_mode = ReferenceDisplayMode::Inline;
        }

        /// Display references as mentions
        pub fn with_display_mention(&mut self) {
            self.display_mode = ReferenceDisplayMode::Mention;
        }

        /// Display references as extra-small cards
        pub fn with_display_card_xs(&mut self) {
            self.display_mode = ReferenceDisplayMode::CardXS;
        }

        /// Display references as small cards
        pub fn with_display_card_sm(&mut self) {
            self.display_mode = ReferenceDisplayMode::CardSM;
        }

        /// Display references as medium cards
        pub fn with_display_card_md(&mut self) {
            self.display_mode = ReferenceDisplayMode::CardMD;
        }

        /// Display references as large cards
        pub fn with_display_card_lg(&mut self) {
            self.display_mode = ReferenceDisplayMode::CardLG;
        }

        /// Display references as extra-large cards
        pub fn with_display_card_xl(&mut self) {
            self.display_mode = ReferenceDisplayMode::CardXL;
        }
      )
    )]
    display_mode: ReferenceDisplayMode,

    #[builder(
      via_mutators(init = None),
      mutators(
        /// Set filter criteria for which items can be referenced
        pub fn with_filter_criteria(&mut self, criteria: Json) {
          self.filter_criteria = Some(criteria);
        }
      )
    )]
    filter_criteria: Option<Json>,
}

impl Default for Reference {
    #[frb(ignore)]
    fn default() -> Self {
        Self::builder().build()
    }
}

impl From<Reference> for FeatureKind {
    fn from(builder: Reference) -> Self {
        FeatureKind::Reference {
            allowed_types: builder.allowed_charts,
            max_references: builder.max_references,
            display_field: builder.display_field,
            display_mode: builder.display_mode,
            filter_criteria: builder.filter_criteria,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_builder_basic() {
        let reference = Reference::default();

        if let FeatureKind::Reference {
            allowed_types,
            max_references,
            display_field,
            display_mode,
            filter_criteria,
        } = reference.into()
        {
            assert_eq!(allowed_types, None);
            assert_eq!(max_references, None);
            assert_eq!(display_field, "title");
            assert_eq!(display_mode, ReferenceDisplayMode::default());
            assert_eq!(filter_criteria, None);
        } else {
            panic!("Expected Reference variant");
        }
    }

    #[test]
    fn test_reference_builder_single_type() {
        let chart_id = ChartID::builtin("person");
        let reference = Reference::builder()
            .add_allowed_chart(chart_id.clone())
            .with_single_reference()
            .with_display_avatar()
            .build()
            .into();

        if let FeatureKind::Reference {
            allowed_types,
            max_references,
            display_mode,
            ..
        } = reference
        {
            assert_eq!(allowed_types, Some(vec![chart_id]));
            assert_eq!(max_references, Some(1));
            assert_eq!(display_mode, ReferenceDisplayMode::Avatar);
        } else {
            panic!("Expected Reference variant");
        }
    }

    #[test]
    fn test_reference_builder_multiple_types() {
        let person_id = ChartID::builtin("person");
        let company_id = ChartID::builtin("company");
        let reference = Reference::builder()
            .with_allowed_charts([person_id.clone(), company_id.clone()])
            .with_max_references(5)
            .with_display_card_sm()
            .with_display_field("name")
            .build()
            .into();

        if let FeatureKind::Reference {
            allowed_types,
            max_references,
            display_field,
            display_mode,
            ..
        } = reference
        {
            assert_eq!(allowed_types, Some(vec![person_id, company_id]));
            assert_eq!(max_references, Some(5));
            assert_eq!(display_field, "name");
            assert_eq!(display_mode, ReferenceDisplayMode::CardSM);
        } else {
            panic!("Expected Reference variant");
        }
    }

    #[test]
    fn test_reference_builder_with_filter() {
        let filter = serde_json::json!({"status": "active"});
        let reference = Reference::builder()
            .with_filter_criteria(filter.clone())
            .with_display_inline()
            .build()
            .into();

        if let FeatureKind::Reference {
            filter_criteria,
            display_mode,
            ..
        } = reference
        {
            assert_eq!(filter_criteria, Some(filter));
            assert_eq!(display_mode, ReferenceDisplayMode::Inline);
        } else {
            panic!("Expected Reference variant");
        }
    }

    #[test]
    fn test_all_display_modes() {
        use crate::mark::ReferenceDisplayMode::*;
        let modes_and_builders = [
            (Inline, Reference::builder().with_display_inline()),
            (Avatar, Reference::builder().with_display_avatar()),
            (CardSM, Reference::builder().with_display_card_sm()),
            (CardMD, Reference::builder().with_display_card_md()),
            (CardLG, Reference::builder().with_display_card_lg()),
        ];

        for (expected_mode, builder) in modes_and_builders {
            let reference = builder.build().into();
            if let FeatureKind::Reference { display_mode, .. } = reference {
                assert_eq!(display_mode, expected_mode);
            } else {
                panic!("Expected Reference variant");
            }
        }
    }
}
