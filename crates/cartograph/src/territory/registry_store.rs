// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{TerritoryError, TerritoryRegistry};
use async_trait::async_trait;

#[async_trait]
pub trait RegistryStore: Send + Sync {
    async fn load(&self) -> Result<TerritoryRegistry, TerritoryError>;
    async fn save(&self, registry: &TerritoryRegistry) -> Result<(), TerritoryError>;
}
