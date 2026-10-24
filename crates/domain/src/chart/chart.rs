use super::{ChartID, Feature, FeatureID};
use crate::{HasIcon, Icon, Json};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::str::FromStr;
use typed_builder::TypedBuilder;

#[derive(
    Debug, Clone, PartialEq, TypedBuilder, Serialize, Deserialize, Type, bedrock::Collection,
)]
#[builder(field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")))]
pub struct Chart {
    /// A unique identifier for the [`Chart`]. This is more thoroughly detailed in
    /// [`ChartID`], but the aim here is to have a compact, unique identifier that allows
    /// Waypoints to reference [`Charts`][Chart] in a way that's not contingent names
    /// (which a user could arbitrarily change).
    #[bedrock(id)]
    #[builder(setter(skip))]
    pub id: ChartID,

    /// A human-readable name for the [`Chart`].
    #[builder(
      via_mutators,
      mutators(
      pub fn with_name(&self, name: impl Into<String>) {
        self.name = name.into();
      }
    ))]
    pub name: String,

    /// A brief description of the [`Chart`].
    #[builder(
      via_mutators,
      mutators(
      pub fn with_description(&self, description: impl Into<String>) {
        self.description = Some(description.into());
      }
    ))]
    pub description: Option<String>,

    /// A [`Icon`] that should be associated with the [`Chart`].
    #[builder(
      via_mutators,
      mutators(
      pub fn with_icon(&self, icon: Icon) {
        self.icon = Some(icon);
      }
    ))]
    pub icon: Option<Icon>,

    /// A mapping of [`FeatureIDs`][`FeatureID`] to [`Feature`] definitions. This describes
    /// the properties that each [`Chart`] should have.
    #[builder(
      via_mutators,
      mutators(
        pub fn add_feature(&self, feature: Feature) {
          self.features.insert(feature.id.clone(), feature);
        }

        pub fn with_features(&self, features: Vec<Feature>) {
          self.features = features.into_iter().map(|f| (f.id.clone(), f)).collect();
        }
      )
    )]
    pub features: IndexMap<FeatureID, Feature>,

    /// Schema-less extra properties. Keys matching `^[0-9a-z]{4}$` are reserved
    /// as [`FeatureID`] slots; namespaced (reverse-DNS) keys cannot collide.
    #[builder(
      via_mutators,
      mutators(
        pub fn with_extra(&self, extra: Json) {
          self.extra = Some(extra)
        }
      )
    )]
    #[specta(type = Option<specta_typescript::Unknown>)]
    pub extra: Option<Json>,
}

impl Chart {
    /// Is this a built-in [`Chart`]?
    pub fn is_builtin(&self) -> bool {
        self.id.is_builtin()
    }

    /// Retrieve a [`Feature`] by its [`FeatureID`].
    pub fn get_feature(&self, feature_id: &FeatureID) -> Option<Feature> {
        self.features.get(feature_id).cloned()
    }
}
