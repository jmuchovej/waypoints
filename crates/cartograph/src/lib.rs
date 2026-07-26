use std::sync::Arc;

use tokio::sync::Mutex;

/// Shared store handle, swapped by territory lifecycle.
pub type Database = Arc<Mutex<Option<store::Store>>>;

pub mod build_info;
pub use build_info::BuildInfo;

pub(crate) mod editor {
    pub(crate) mod error;
    mod handle;
    mod history;
    pub(crate) mod maneuver;
    pub(crate) mod mutation;
    pub(crate) mod projection;
    pub(crate) mod rich_text;
    pub(crate) mod service;
    #[cfg(test)]
    mod tests;
    pub(crate) mod types;

    pub use error::{EditorError, EditorResult};
    pub use handle::EditorHandle;
    pub use maneuver::{Maneuver, ManeuverTarget};
    pub use types::{
        CreateResult, CursorState, MutationResult, RenderNode, SplitResult, UndoRedoResult,
    };

    pub(crate) use service::EditorService;
}
pub use editor::{
    CreateResult, CursorState, EditorError, EditorHandle, EditorResult, Maneuver, ManeuverTarget,
    MutationResult, RenderNode, SplitResult, UndoRedoResult,
};

pub mod chart {
    mod handle;

    pub use handle::ChartHandle;
}

pub mod mark {
    mod error;
    mod handle;

    pub use error::{MarkError, MarkResult};
    pub use handle::MarkHandle;
}

pub mod territory {
    mod error;
    mod handle;
    pub(crate) mod manager;
    mod registry_store;
    mod types;

    pub use error::{TerritoryError, TerritoryResult};
    pub use handle::TerritoryHandle;
    pub use registry_store::RegistryStore;
    pub use types::{Territory, TerritoryKind, TerritoryRegistry};

    pub(crate) use manager::TerritoryManager;
}

pub mod sync {
    mod error;
    mod service;
    mod setup_code;
    mod types;

    pub use error::{SyncError, SyncResult};
    pub use service::SyncService;
    pub use setup_code::{SetupCode, classify, encode_pair_url};
    pub use types::{IdentityCreated, PairingCode, StationStatus, SyncDevice, SyncStatus};
}

mod runtime;
pub use runtime::WaypointsRuntime;
