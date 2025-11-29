use super::{Content, Landmark, MarkData, MarkID};
use crate::Json;
use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, PartialEq, TypedBuilder, Serialize, Deserialize)]
#[builder(
  field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")),
  builder_type(attributes(#[frb(ignore)])),
)]
#[frb(non_opaque, unignore)]
pub struct Mark {
    /// A unique identifier for the [`Mark`]. This is more thoroughly detailed in
    /// [`MarkID`], but the aim here is to have a high-capacity, unique identifier that
    /// allows Waypoints to reference [`Marks`][Mark] in a way that's not contingent the
    /// data held by the [`Mark`] (which a user could arbitrarily change).
    #[builder(default, setter(skip))]
    pub id: MarkID,

    /// The actual data held by the [`Mark`]. This will take the form of either
    /// [`MarkData::Content`] or [`MarkData::Landmark`].
    #[builder(
      via_mutators,
      mutators(
        pub fn with_landmark(&self, landmark: Landmark) {
          self.data = MarkData::Landmark(landmark)
        }

        pub fn with_content(&self, content: Content) {
          self.data = MarkData::Content(content)
        }
      )
    )]
    pub data: MarkData,

    /// Any extra properties that are attached to this [`Mark`]. It's deliberately
    /// schema-less. In the future, it will will support dynamic rendering, too!
    // TODO remove this above comment once dynamic rendering is supported.
    #[builder(
      via_mutators,
      mutators(
        pub fn with_extra(&self, extra: Json) {
          self.extra = Some(extra)
        }
      )
    )]
    pub extra: Option<Json>,
}

impl Mark {
    /// Is this a [`Content`] Mark?
    pub fn is_content(&self) -> bool {
        self.data.is_content()
    }

    /// Is this a [`Landmark`] Mark?
    pub fn is_landmark(&self) -> bool {
        self.data.is_landmark()
    }

    /// Retrieves the `type_name` of the contained [`MarkData`]. This forwards its call to
    /// the contained [`MarkData::type_name`].
    pub fn type_name(&self) -> String {
        self.data.type_name()
    }
}
