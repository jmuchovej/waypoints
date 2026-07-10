use super::{Content, Landmark, MarkData};
use crate::{DomainError, DomainResult, Json};
use chrono::NaiveDateTime;
use derive_more::{Debug, Display, FromStr};
use rand::Rng;
use serde::{Deserialize, Serialize};
use specta::Type;
use typed_builder::TypedBuilder;
use uuid::Uuid;

/// The current UTC time, truncated to milliseconds — the engine's `timestamp`
/// columns store millisecond precision and reject anything finer.
pub fn now_ms() -> NaiveDateTime {
    chrono::DateTime::from_timestamp_millis(chrono::Utc::now().timestamp_millis())
        .expect("current time is a valid millisecond timestamp")
        .naive_utc()
}

/// Before-save hook: stamp `updated_at` on every write. `created_at` is set at
/// construction (defaulting to now) and left alone.
fn touch(mut mark: Mark, _op: bedrock::WriteOp) -> Mark {
    mark.updated_at = now_ms();
    mark
}

/// Unique identifier for a [`Mark`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Type, bedrock::Scalar,
)]
#[display("{}", self.inner().to_string().to_lowercase())]
#[debug("MarkID({})", self.inner().to_string().to_lowercase())]
#[serde(transparent)]
pub struct MarkID(pub Uuid);

impl MarkID {
    /// Create a new, random, [`Mark`] id (which is a [`Uuid::new_v4`]).
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Retrieve the inner [`Uuid`] within the [`MarkID`].
    pub fn inner(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for MarkID {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for MarkID {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(
    Debug, Clone, PartialEq, TypedBuilder, Serialize, Deserialize, Type, bedrock::Collection,
)]
#[builder(field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")))]
#[bedrock(before_save = touch)]
pub struct Mark {
    /// A unique identifier for the [`Mark`]. This is more thoroughly detailed in
    /// [`MarkID`], but the aim here is to have a high-capacity, unique identifier that
    /// allows Waypoints to reference [`Marks`][Mark] in a way that's not contingent the
    /// data held by the [`Mark`] (which a user could arbitrarily change).
    #[bedrock(id)]
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
    #[specta(type = Option<specta_typescript::Unknown>)]
    pub extra: Option<Json>,

    /// When the [`Mark`] was created. Defaults to now; a caller (e.g. an import)
    /// may set it explicitly.
    #[builder(default = now_ms())]
    pub created_at: NaiveDateTime,

    /// When the [`Mark`] was last written. Stamped on every save by the
    /// before-save hook.
    #[builder(default = now_ms())]
    pub updated_at: NaiveDateTime,

    /// Soft-delete tombstone: set when the [`Mark`] is removed; live reads
    /// exclude it.
    pub deleted_at: Option<NaiveDateTime>,
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
