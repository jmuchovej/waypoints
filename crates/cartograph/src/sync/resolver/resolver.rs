use crate::editor::{EditorResult, Maneuver};
use domain::{DeviceID, HLC};
use std::future::Future;
use store::DatabaseConnection;

/// How an incoming remote op was reconciled with local state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The incoming write replaced local state wholesale (it won).
    Applied,
    /// Local state won; the incoming op was discarded.
    Dominated,
    /// The incoming op was merged with local state — both contributions
    /// survive. No v1 resolver produces this; it's the CRDT outcome (a
    /// delta merge has no loser), reserved so conflict telemetry and a
    /// future "you may have lost edits" UI can tell the cases apart.
    Merged,
}

/// Strategy for reconciling an incoming remote op with local state.
///
/// v1 ships [`LastWriteResolver`](super::lww::LastWriteResolver) only.
/// When char-level text CRDTs land they arrive as a sibling resolver
/// routed per payload kind — structural ops stay LWW forever; only
/// `TextPayload::CrdtOps` merges — not as a wholesale replacement.
#[async_trait::async_trait]
pub trait ConflictResolver: Send + Sync {
    async fn resolve<'r>(
        &'r self,
        db: &'r DatabaseConnection,
        op: &'r Maneuver,
        hlc: HLC,
        device_id: DeviceID,
    ) -> EditorResult<Resolution>;
}
