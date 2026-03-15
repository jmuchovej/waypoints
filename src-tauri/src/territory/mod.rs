pub(crate) mod db;
mod error;
mod manager;
mod registry;

pub(crate) use error::{TerritoryError, TerritoryResult};
pub(crate) use manager::TerritoryManager;
pub(crate) use registry::{Territory, TerritoryKind, TerritoryRegistry};
