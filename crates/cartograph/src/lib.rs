use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

/// Shared database connection, swapped by territory lifecycle.
pub type Database = Arc<Mutex<Option<DatabaseConnection>>>;

pub(crate) mod editor {
    pub(crate) mod error;
    mod handle;
    mod history;
    pub(crate) mod mutation;
    pub(crate) mod ops;
    pub(crate) mod projection;
    pub(crate) mod rich_text;
    pub(crate) mod service;
    #[cfg(test)]
    mod tests;
    pub(crate) mod types;

    pub use error::{EditorError, EditorResult};
    pub use handle::EditorHandle;
    pub use types::{
        CreateResult, CursorState, MutationResult, RenderNode, SplitResult, UndoRedoResult,
    };

    pub(crate) use service::EditorService;
}

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

pub mod sync {
    mod coordinator;
    mod envelope;
    mod lww;
    mod relay;

    pub use envelope::TextPayload;
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

mod runtime;

pub use editor::{
    CreateResult, CursorState, EditorError, EditorHandle, EditorResult, MutationResult, RenderNode,
    SplitResult, UndoRedoResult,
};
pub use runtime::WaypointsRuntime;
