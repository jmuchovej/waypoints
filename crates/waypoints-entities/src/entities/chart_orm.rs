//! Schema definitions and field types for Thing schemas.
//!
//! This module contains all the schema-related code:
//! - Field types and validation logic
//! - Schema definitions (ThingSchema)
//! - Schema-related utilities and registries
//!
//! ## Page-Based Inheritance Model
//!
//! All Things in the system are conceptually Pages with additional fields.
//! The Page type is the fundamental building block, providing the base
//! structure that all other types extend:
//!
//! - **Page**: Base type with title, icon, and children fields
//! - **Task**: Page + completion status, due date, priority, etc.
//! - **Note**: Page + content, tags, source URL
//! - **Project**: Page + timeline, team members, status
//! - ... and so on
//!
//! This inheritance is made explicit through the `extend_page()` and
//! `extend_page_builtin()` methods, ensuring consistency across all types.
//!
//! This is separate from thing instances - schemas define what's possible,
//! while Thing instances store actual data.
use sea_orm::{
    ActiveModelBehavior, DbErr, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EnumIter,
    IntoActiveModel, PrimaryKeyTrait, prelude::Json,
};
use serde::{Deserialize, Serialize};
use waypoints_domain::{
    Icon,
    chart::{Chart, ChartID, Feature},
};

/// Charts define the various kinds of [`Landmarks`][`crate::landmark::Landmark`] you can have within Waypoints.
///
/// All charts inherit from the Page Chart, which provides the basic features
/// (title, icon, children) that every Landmark has. Custom charts extend Page
/// by adding their own specialized features.
///
/// Users can create custom `Charts` to define new kinds of Landmarks.
///
/// # Examples
///
/// ## Creating a Basic Chart
///
/// ```
/// use waypoints::chart::Chart;
/// use waypoints::feature::{Feature, FeatureKind};
///
/// let mut recipe_chart = Chart::new("Recipe".to_string())
///     .with_description("A cooking recipe with ingredients and instructions")
///     .with_icon("👨‍🍳");
///
/// // Add custom features
/// recipe_chart = recipe_chart.add_feature(Feature::new(
///     "Prep Time".to_string(),
///     FeatureKind::number()
/// ));
///
/// recipe_chart = recipe_chart.add_feature(Feature::new(
///     "Difficulty".to_string(),
///     FeatureKind::unified_select_builder()
///         .with_option("Easy")
///         .with_option("Medium")
///         .with_option("Hard")
///         .single_selection()
///         .build()
/// ));
///
/// assert_eq!(recipe_chart.name, "Recipe");
/// assert_eq!(recipe_chart.features.len(), 2);
/// assert!(!recipe_chart.is_builtin());
/// ```
///
/// ## Extending Page Chart
///
/// ```
/// use waypoints::chart::Chart;
/// use waypoints::feature::{Feature, FeatureKind};
///
/// // Create a Meeting Notes chart that extends Page
/// let mut meeting_notes = Chart::extend_page("Meeting Notes".to_string())
///     .with_description("Notes from team meetings")
///     .with_icon("📝");
///
/// // Add meeting-specific features
/// meeting_notes = meeting_notes.add_feature(Feature::new(
///     "Date".to_string(),
///     FeatureKind::date()
/// ));
///
/// meeting_notes = meeting_notes.add_feature(Feature::new(
///     "Attendees".to_string(),
///     FeatureKind::unified_select_builder()
///         .allow_multiple()
///         .build()
/// ));
///
/// // Base fields (title, icon, children) are automatically available
/// // on any Landmark created from this chart
/// assert_eq!(meeting_notes.features.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "charts", rename_all = "kebab-case")]
pub struct Model {
    /// Unique identifier for this type
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this type represents
    pub description: Option<String>,
    /// Typed icon with rendering properties (preferred)
    pub icon: Option<Json>,
    /// Field definitions that define the schema
    pub features: Json,
}

#[derive(Debug, Copy, Clone, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    // fn before_save(self: ActiveModel, insert: bool) -> Result<Self, DbErr> {
    //     use sea_orm::ActiveValue::Set;

    //     Ok(self)
    // }
}

impl From<Model> for Chart {
    fn from(model: Model) -> Self {
        let mut builder = Chart::builder().with_name(model.name);

        if let Some(description) = model.description {
            builder = builder.with_description(description);
        }

        if let Some(icon) = model.icon {
            let icon: Icon = serde_json::from_value(icon).unwrap();
            builder = builder.with_icon(icon);
        }

        for (id, feature) in model.features.as_object().unwrap() {
            let feature: Feature = serde_json::from_value(feature.clone()).unwrap();
            builder = builder.add_feature(feature);
        }

        let chart: Self = builder.build();

        Self {
            id: model.id.parse::<ChartID>().unwrap(),
            ..chart
        }
    }
}
