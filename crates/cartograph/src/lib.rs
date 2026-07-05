use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;

/// Shared database connection, swapped by territory lifecycle.
pub type Database = Arc<Mutex<Option<DatabaseConnection>>>;

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

pub mod sync {
    mod resolver {
        pub mod lww;
        pub mod resolver;

        pub use lww::LastWriteResolver;
        pub use resolver::{ConflictResolver, Resolution};
    }
    mod coordinator;
    mod driver;
    mod envelope;
    mod relay;

    pub use coordinator::{ApplyOutcome, SkipReason, SyncCoordinator, touched_marks};
    pub use driver::{SyncOutcome, sync_once};
    pub use envelope::{Envelope, TextPayload};
    pub use relay::{
        FetchPage, HttpRelayClient, InMemoryRelayClient, NoopRelayClient, RelayClient,
        RelayError, SequencedEnvelope, UploadAck,
    };
    pub use resolver::{ConflictResolver, LastWriteResolver, Resolution};
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
pub use runtime::WaypointsRuntime;
