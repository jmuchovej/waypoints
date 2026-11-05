// SPDX-FileCopyrightText: 2025 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of Waypoints.
//
// Waypoints is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Waypoints is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.
//
// Additional permission under GNU AGPL version 3 section 7: this Program
// may be conveyed through application stores on the terms set out in
// docs/licensing/COPYING.exceptions.

use std::sync::Arc;
use tokio::sync::Mutex;

// Features are additive and flavors are not: one shipped flavor per build.
#[cfg(all(feature = "nightly", feature = "preview"))]
compile_error!(
    "cartograph: the `nightly` and `preview` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "nightly", feature = "stable"))]
compile_error!(
    "cartograph: the `nightly` and `stable` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "preview", feature = "stable"))]
compile_error!(
    "cartograph: the `preview` and `stable` flavors are both enabled; a build takes exactly one"
);

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
    mod publish;
    mod service;
    mod setup_code;
    mod types;

    pub use arbitration::ArbitrationKey;
    pub use error::{SyncError, SyncResult};
    pub use fold::{
        FoldError, FoldReport, KeyedOp, PAGE_V1, decode_page_v1, encode_page_v1, fold,
        fold_incremental, fold_new_notices, keyed_ops, sort_canonical, target_key, winners,
    };
    pub use publish::{choose_track_landmark, publish_queued};
    pub use service::SyncService;
    pub use setup_code::{SetupCode, classify, encode_pair_url};
    pub use types::{
        IdentityCreated, PairingCode, StationStatus, SyncDevice, SyncState, SyncStatus,
    };
}
pub use sync::{
    ArbitrationKey, FoldError, FoldReport, IdentityCreated, KeyedOp, PAGE_V1, PairingCode,
    SetupCode, StationStatus, SyncDevice, SyncError, SyncResult, SyncService, SyncState,
    SyncStatus, choose_track_landmark, classify, decode_page_v1, encode_page_v1, encode_pair_url,
    fold, fold_incremental, fold_new_notices, keyed_ops, publish_queued, sort_canonical,
    target_key, winners,
};

mod runtime;
pub use runtime::WaypointsRuntime;
