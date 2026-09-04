use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared store handle, swapped by territory lifecycle.
pub type Database = Arc<Mutex<Option<store::Store>>>;

pub(crate) mod build_info;
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
    pub use maneuver::{Maneuver, ManeuverTarget, TextPayload};
    pub use types::{
        CreateResult, CursorState, MutationResult, RenderNode, SplitResult, UndoRedoResult,
    };

    pub(crate) use history::CommandHistory;
    pub(crate) use service::EditorService;
}
pub(crate) use editor::{CommandHistory, EditorService, TextPayload};
pub use editor::{
    CreateResult, CursorState, EditorError, EditorHandle, EditorResult, Maneuver, ManeuverTarget,
    MutationResult, RenderNode, SplitResult, UndoRedoResult,
};

pub(crate) mod chart {
    mod handle;

    pub use handle::ChartHandle;
}
pub use chart::ChartHandle;

pub(crate) mod mark {
    mod error;
    mod handle;

    pub use error::{MarkError, MarkResult};
    pub use handle::MarkHandle;
}
pub use mark::{MarkError, MarkHandle, MarkResult};

pub(crate) mod territory {
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
pub use territory::{
    RegistryStore, Territory, TerritoryError, TerritoryHandle, TerritoryKind, TerritoryRegistry,
    TerritoryResult,
};

pub(crate) mod sync {
    mod arbitration;
    mod error;
    mod fold;
    mod service;
    mod setup_code;
    mod types;

    pub use arbitration::ArbitrationKey;
    pub use error::{SyncError, SyncResult};
    pub use fold::{
        FoldError, KeyedOp, PAGE_V1, decode_page_v1, encode_page_v1, fold, keyed_ops,
        sort_canonical, winners,
    };
    pub use service::SyncService;
    pub use setup_code::{SetupCode, classify, encode_pair_url};
    pub use types::{IdentityCreated, PairingCode, StationStatus, SyncDevice, SyncStatus};
}
pub use sync::{
    ArbitrationKey, FoldError, IdentityCreated, KeyedOp, PAGE_V1, PairingCode, SetupCode,
    StationStatus, SyncDevice, SyncError, SyncResult, SyncService, SyncStatus, classify,
    decode_page_v1, encode_page_v1, encode_pair_url, fold, keyed_ops, sort_canonical, winners,
};

mod runtime;
pub use runtime::WaypointsRuntime;
