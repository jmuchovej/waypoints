use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TerritoryKind {
    Managed,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Territory {
    pub id: Uuid,
    pub name: String,
    pub kind: TerritoryKind,
    pub path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_opened_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerritoryRegistry {
    pub registry: Vec<Territory>,
    pub last_active_id: Option<Uuid>,
}

impl Default for TerritoryRegistry {
    fn default() -> Self {
        Self {
            registry: Vec::new(),
            last_active_id: None,
        }
    }
}
