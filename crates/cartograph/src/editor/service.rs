use std::collections::HashMap;
use std::sync::Arc;

use domain::mark::{Content, Mark, MarkData, MarkID, RichText};
use store::DatabaseConnection;
use tokio::sync::{Mutex, OwnedMutexGuard};

use super::error::{EditorError, EditorResult};
use super::history::CommandHistory;
use super::types::{CreateResult, MutationResult, RenderNode, SplitResult, UndoRedoResult};
use super::{mutation, projection};

use crate::Database;
use crate::sync::{SyncCoordinator, touched_marks};

/// The editor service holds a shared database connection, per-landmark undo
/// histories, and the sync coordinator every mutation logs through.
///
/// The [`Database`](crate::Database) Arc is shared with `TerritoryManager` so territory switches
/// only need to swap the inner `Option` — no EditorService reconstruction required.
pub struct EditorService {
    db: Database,
    histories: HashMap<MarkID, CommandHistory>,
    /// Lazily loaded on first mutation (needs a live territory DB) and
    /// reset on territory switch — device identity is per-territory.
    sync: Arc<Mutex<Option<SyncCoordinator>>>,
}

/// Get or create a `CommandHistory` for `landmark_id` from the map.
/// Returns a mutable reference that doesn't borrow `self`, allowing
/// `db` to be borrowed simultaneously.
fn history_entry(
    histories: &mut HashMap<MarkID, CommandHistory>,
    landmark_id: MarkID,
) -> &mut CommandHistory {
    histories
        .entry(landmark_id)
        .or_insert_with(CommandHistory::new)
}

/// Lock the coordinator, loading it from the device table on first use.
async fn sync_guard(
    sync: &Arc<Mutex<Option<SyncCoordinator>>>,
    db: &DatabaseConnection,
) -> EditorResult<OwnedMutexGuard<Option<SyncCoordinator>>> {
    let mut guard = sync.clone().lock_owned().await;
    if guard.is_none() {
        *guard = Some(SyncCoordinator::load(db).await?);
    }
    Ok(guard)
}

impl EditorService {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            histories: HashMap::new(),
            sync: Arc::new(Mutex::new(None)),
        }
    }

    /// Lock the shared Arc, clone the connection (cheap pool handle), release lock.
    async fn db(&self) -> EditorResult<DatabaseConnection> {
        self.db
            .lock()
            .await
            .clone()
            .ok_or(EditorError::NoActiveTerritory)
    }

    /// Clear all undo histories and sync state (e.g. on territory switch).
    pub fn clear_histories(&mut self) {
        self.histories.clear();
        // Fresh territory = fresh device identity; drop the coordinator so
        // it reloads lazily from the new database. Replacing the Arc is
        // fine while nothing else holds a clone (the relay driver will,
        // once Phase 6 lands — revisit then).
        self.sync = Arc::new(Mutex::new(None));
    }

    /// Fetch a single mark by ID.
    pub async fn get_mark(&self, mark_id: MarkID) -> EditorResult<Mark> {
        let db = self.db().await?;
        store::query::mark::get(&db, mark_id)
            .await?
            .ok_or(EditorError::NotFound(mark_id.inner()))
    }

    /// List all landmark marks (equivalent to old `mark_list`).
    pub async fn mark_list(&self) -> EditorResult<Vec<Mark>> {
        let db = self.db().await?;
        Ok(store::query::mark::list(&db, Some("landmark"), None).await?)
    }

    /// Project a landmark into a flat block list.
    pub async fn project_landmark(
        &self,
        landmark_id: MarkID,
        max_depth: Option<u32>,
    ) -> EditorResult<Vec<RenderNode>> {
        let db = self.db().await?;
        projection::project_landmark(&db, landmark_id, max_depth).await
    }

    pub async fn update_mark(
        &mut self,
        id: MarkID,
        content: Content,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::update_mark(&db, h, sync, id, content, landmark_id).await
    }

    pub async fn delete_mark(
        &mut self,
        id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::delete_mark(&db, h, sync, id, landmark_id).await
    }

    pub async fn move_marks(
        &mut self,
        parent_id: MarkID,
        child_ids: &[MarkID],
        after_id: Option<MarkID>,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::move_marks(&db, h, sync, parent_id, child_ids, after_id, landmark_id).await
    }

    pub async fn update_mark_title(&self, id: MarkID, title: RichText) -> EditorResult<()> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        mutation::update_mark_title(&db, sync, id, title).await
    }

    pub async fn update_mark_reading(
        &self,
        id: MarkID,
        feature_id: domain::chart::FeatureID,
        reading: domain::mark::Reading,
    ) -> EditorResult<Mark> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        mutation::update_mark_reading(&db, sync, id, feature_id, reading).await
    }

    pub async fn create_mark_after(
        &mut self,
        after_id: MarkID,
        parent_id: MarkID,
        data: MarkData,
        landmark_id: MarkID,
    ) -> EditorResult<CreateResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::create_mark_after(&db, h, sync, after_id, parent_id, data, landmark_id).await
    }

    pub async fn wrap_in_compound(
        &mut self,
        compound_data: MarkData,
        child_ids: &[MarkID],
        after_id: Option<MarkID>,
        parent_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<CreateResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::wrap_in_compound(
            &db,
            h,
            sync,
            compound_data,
            child_ids,
            after_id,
            parent_id,
            landmark_id,
        )
        .await
    }

    pub async fn update_mark_text(
        &mut self,
        mark_id: MarkID,
        rich_text: RichText,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::update_mark_text(&db, h, sync, mark_id, rich_text, landmark_id).await
    }

    pub async fn split_mark(
        &mut self,
        mark_id: MarkID,
        offset: u32,
        landmark_id: MarkID,
    ) -> EditorResult<SplitResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::split_mark(&db, h, sync, mark_id, offset as usize, landmark_id).await
    }

    pub async fn merge_marks(
        &mut self,
        target_id: MarkID,
        source_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::merge_marks(&db, h, sync, target_id, source_id, landmark_id).await
    }

    pub async fn delete_marks(
        &mut self,
        mark_ids: &[MarkID],
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::delete_marks(&db, h, sync, mark_ids, landmark_id).await
    }

    pub async fn reparent_mark(
        &mut self,
        mark_id: MarkID,
        new_parent_id: MarkID,
        after_id: Option<MarkID>,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::reparent_mark(&db, h, sync, mark_id, new_parent_id, after_id, landmark_id).await
    }

    pub async fn undo(&mut self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        match h.undo(&db).await? {
            Some((cursor, ops)) => {
                // An undo mutates the database like any other command
                // group — it must reach other devices too.
                let marks = touched_marks(&ops);
                sync.log_local(&db, ops, marks).await?;
                let blocks = projection::project_landmark(&db, landmark_id, None).await?;
                Ok(Some(UndoRedoResult {
                    blocks,
                    cursor: Some(cursor),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn redo(&mut self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        let db = self.db().await?;
        let mut guard = sync_guard(&self.sync, &db).await?;
        let sync = guard.as_mut().expect("coordinator loaded above");
        let h = history_entry(&mut self.histories, landmark_id);
        match h.redo(&db).await? {
            Some((cursor, ops)) => {
                let marks = touched_marks(&ops);
                sync.log_local(&db, ops, marks).await?;
                let blocks = projection::project_landmark(&db, landmark_id, None).await?;
                Ok(Some(UndoRedoResult {
                    blocks,
                    cursor: Some(cursor),
                }))
            }
            None => Ok(None),
        }
    }
}
