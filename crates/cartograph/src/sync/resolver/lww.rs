use super::{ConflictResolver, Resolution};
use crate::editor::{EditorResult, Maneuver, ManeuverTarget};
use domain::{DeviceID, HLC};
use store::{
    DatabaseConnection,
    query::{mark, terrain},
};

/// Last-write-wins: an incoming maneuver applies IFF its
/// `(hlc, device_id)` key beats the target row's stored clock.
///
/// The skip condition is `stored >= incoming`: `Greater` means the stored
/// write wins, `Equal` means this exact write was already applied (same
/// origin). The `device_id` tiebreak is what prevents split-brain when two
/// devices produce identical HLCs — UUID-byte order is arbitrary but
/// deterministic everywhere.
pub struct LastWriteResolver;

#[async_trait::async_trait]
impl ConflictResolver for LastWriteResolver {
    async fn resolve<'r>(
        &'r self,
        db: &'r DatabaseConnection,
        op: &'r Maneuver,
        hlc: HLC,
        device_id: DeviceID,
    ) -> EditorResult<Resolution> {
        let stored = match op.target() {
            ManeuverTarget::Mark(id) => mark::get_row_clock(db, id).await?,
            ManeuverTarget::Terrain(id) => terrain::get_row_clock(db, id).await?,
        };
        if stored.is_some_and(|key| key >= (hlc, device_id)) {
            return Ok(Resolution::Dominated);
        }

        op.apply(db).await?;
        match op.target() {
            ManeuverTarget::Mark(id) => mark::set_row_clock(db, id, hlc, device_id).await?,
            ManeuverTarget::Terrain(id) => terrain::set_row_clock(db, id, hlc, device_id).await?,
        }
        Ok(Resolution::Applied)
    }
}
