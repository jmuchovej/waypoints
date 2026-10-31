use derive_more::{Debug, Display};
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Display, Type,
)]
#[display("{}", self.inner().to_string().to_lowercase())]
#[debug("AccountID({})", self.inner().to_string().to_lowercase())]
#[serde(transparent)]
pub struct AccountID(pub Uuid);

impl AccountID {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn inner(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for AccountID {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for AccountID {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Display, Type,
)]
#[display("{}", self.inner().to_string().to_lowercase())]
#[debug("RealmID({})", self.inner().to_string().to_lowercase())]
#[serde(transparent)]
pub struct RealmID(pub Uuid);

impl RealmID {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn inner(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for RealmID {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for RealmID {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Display, Type,
)]
#[display("{}", self.inner().to_string().to_lowercase())]
#[debug("DeviceID({})", self.inner().to_string().to_lowercase())]
#[serde(transparent)]
pub struct DeviceID(pub Uuid);

impl DeviceID {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn inner(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for DeviceID {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for DeviceID {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Display, Type,
)]
#[display("{}", self.inner().to_string().to_lowercase())]
#[debug("OpID({})", self.inner().to_string().to_lowercase())]
#[serde(transparent)]
pub struct OpID(pub Uuid);

impl OpID {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn inner(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for OpID {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for OpID {
    fn default() -> Self {
        Self::new()
    }
}
