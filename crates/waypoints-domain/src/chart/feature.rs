//! Feature implementations for chart validation.
//!
//! Each feature type handles its own validation logic, default values,
//! and JSON conversion. Features define the structure of data fields
//! while validation logic is handled in feature_validators.
use super::{ChartID, FeatureID, FeatureKind, GlyphID};
use crate::{DomainResult, Json};
use chrono::{NaiveDate, NaiveTime};
use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// A feature defines a field within a [`Chart`][C] that [`Landmarks`][M:L] can have values for.
///
/// Features have a unique ID ([`FeatureID`][F.id]), a human-readable title, and a
/// [`FeatureKind`][FK] that determines the kind of data that can be stored (and its validation).
///
/// [C]: super::Chart
/// [FK]: super::FeatureKind
/// [F.id]: super::FeatureID
/// [M:L]: crate::mark::Landmark
#[derive(Debug, Clone, PartialEq, TypedBuilder, Serialize, Deserialize)]
#[builder(
  field_defaults(setter(strip_option(ignore_invalid), prefix = "with_")),
  builder_type(attributes(#[frb(ignore)])),
)]
#[frb(non_opaque, unignore)]
pub struct Feature {
    #[builder(default, setter(skip))]
    pub id: FeatureID,

    #[builder(
      via_mutators,
      mutators(
        pub fn with_name(&self, name: impl Into<String>) {
          self.name = name.into();
        }
      )
    )]
    pub name: String,

    #[builder(
      via_mutators,
      mutators(
        pub fn with_kind(&self, kind: FeatureKind) {
          self.kind = kind;
        }
      )
    )]
    pub kind: FeatureKind,
    #[builder(
      via_mutators,
      mutators(
        pub fn with_description(&self, description: impl Into<String>) {
          self.description = Some(description.into());
        }
      )
    )]
    pub description: Option<String>,
    #[builder(default = false, via_mutators(init = false), mutators(
      pub fn required(mut self) {
        self.required = true;
      }
    ))]
    pub required: bool,
}
