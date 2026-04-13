use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use domain::mark::{Content, Mark, MarkData, MarkID, RichText};
use store::DatabaseConnection;

use crate::error::{EditorError, EditorResult};
use crate::history::CommandHistory;
use crate::types::{CreateResult, MutationResult, RenderNode, SplitResult, UndoRedoResult};
use crate::{mutation, projection};

pub type SharedDatabase = Arc<Mutex<Option<DatabaseConnection>>>;

/// The editor service holds a shared database connection and per-landmark undo histories.
///
/// The [`SharedDatabase`] Arc is shared with `TerritoryManager` so territory switches
/// only need to swap the inner `Option` — no EditorService reconstruction required.
pub struct EditorService {
    db: SharedDatabase,
    histories: HashMap<MarkID, CommandHistory>,
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

impl EditorService {
    pub fn new(db: SharedDatabase) -> Self {
        Self {
            db,
            histories: HashMap::new(),
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

    /// Clear all undo histories (e.g. on territory switch).
    pub fn clear_histories(&mut self) {
        self.histories.clear();
    }

    // ----- Read operations -----

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
        Ok(store::query::mark::list(&db, Some("landmark")).await?)
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

    // ----- Write operations -----

    pub async fn update_mark(
        &mut self,
        id: MarkID,
        content: Content,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::update_mark(&db, h, id, content, landmark_id).await
    }

    pub async fn delete_mark(
        &mut self,
        id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::delete_mark(&db, h, id, landmark_id).await
    }

    pub async fn move_marks(
        &mut self,
        parent_id: MarkID,
        child_ids: &[MarkID],
        after_id: Option<MarkID>,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::move_marks(&db, h, parent_id, child_ids, after_id, landmark_id).await
    }

    pub async fn update_mark_title(&self, id: MarkID, title: RichText) -> EditorResult<()> {
        let db = self.db().await?;
        mutation::update_mark_title(&db, id, title).await
    }

    pub async fn update_mark_reading(
        &self,
        id: MarkID,
        feature_id: domain::chart::FeatureID,
        reading: domain::mark::Reading,
    ) -> EditorResult<Mark> {
        let db = self.db().await?;
        mutation::update_mark_reading(&db, id, feature_id, reading).await
    }

    pub async fn create_mark_after(
        &mut self,
        after_id: MarkID,
        parent_id: MarkID,
        data: MarkData,
        landmark_id: MarkID,
    ) -> EditorResult<CreateResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::create_mark_after(&db, h, after_id, parent_id, data, landmark_id).await
    }

    pub async fn update_mark_text(
        &mut self,
        mark_id: MarkID,
        rich_text: RichText,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::update_mark_text(&db, h, mark_id, rich_text, landmark_id).await
    }

    pub async fn split_mark(
        &mut self,
        mark_id: MarkID,
        offset: u32,
        landmark_id: MarkID,
    ) -> EditorResult<SplitResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::split_mark(&db, h, mark_id, offset as usize, landmark_id).await
    }

    pub async fn merge_marks(
        &mut self,
        target_id: MarkID,
        source_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::merge_marks(&db, h, target_id, source_id, landmark_id).await
    }

    pub async fn delete_marks(
        &mut self,
        mark_ids: &[MarkID],
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::delete_marks(&db, h, mark_ids, landmark_id).await
    }

    pub async fn reparent_mark(
        &mut self,
        mark_id: MarkID,
        new_parent_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::reparent_mark(&db, h, mark_id, new_parent_id, landmark_id).await
    }

    pub async fn undo(&mut self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        let cursor = h.undo(&db).await?;
        match cursor {
            Some(c) => {
                let blocks = projection::project_landmark(&db, landmark_id, None).await?;
                Ok(Some(UndoRedoResult {
                    blocks,
                    cursor: Some(c),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn redo(&mut self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        let cursor = h.redo(&db).await?;
        match cursor {
            Some(c) => {
                let blocks = projection::project_landmark(&db, landmark_id, None).await?;
                Ok(Some(UndoRedoResult {
                    blocks,
                    cursor: Some(c),
                }))
            }
            None => Ok(None),
        }
    }
}
