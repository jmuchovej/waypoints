use crate::{DomainError, DomainResult};
use derive_more::{Debug, Display, FromStr};
use rand::Rng;
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// Unique identifier for a [`Mark`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Type)]
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

#[allow(clippy::module_inception)]
pub mod mark;
pub mod mark_data;
pub mod reading;
pub mod rich_reference;
pub mod rich_text;

pub use mark::Mark;
pub use mark_data::{Content, Landmark, MarkData};
pub use reading::Reading;
pub use rich_reference::{ReferenceDisplayMode, RichReference};
pub use rich_text::{InlineMark, RichText, TextMarkKind, TextSegment};

pub mod reading_validators;

pub mod builders {
    use super::mark;
    pub use mark::MarkBuilder;
}
