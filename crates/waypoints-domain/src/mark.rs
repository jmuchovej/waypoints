use crate::{DomainError, DomainResult};
use derive_more::{Debug, Display, FromStr};
use flutter_rust_bridge::frb;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a [`Mark`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
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
    #[frb(ignore)]
    fn default() -> Self {
        Self::new()
    }
}
