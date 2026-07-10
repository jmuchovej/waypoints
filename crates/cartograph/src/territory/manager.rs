use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use super::{Territory, TerritoryKind};

pub struct TerritoryManager {
    managed_root: PathBuf,
    pub active_db: Arc<Mutex<Option<store::Store>>>,
    pub active_id: Arc<Mutex<Option<Uuid>>>,
}

impl TerritoryManager {
    pub fn new(managed_root: PathBuf) -> Self {
        Self {
            managed_root,
            active_db: Arc::new(Mutex::new(None)),
            active_id: Arc::new(Mutex::new(None)),
        }
    }

    pub fn managed_path(&self, id: Uuid) -> PathBuf {
        self.managed_root.join(id.to_string())
    }

    pub fn entry_path(&self, entry: &Territory) -> PathBuf {
        use TerritoryKind::{External, Managed};
        match entry.kind {
            Managed => self.managed_path(entry.id),
            External => PathBuf::from(
                entry
                    .path
                    .as_deref()
                    .expect("external workspace must have a path"),
            ),
        }
    }
}
