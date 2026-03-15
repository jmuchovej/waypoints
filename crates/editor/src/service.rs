use std::collections::HashMap;

use domain::mark::{Content, Mark, MarkData, MarkID, RichText};
use store::DatabaseConnection;

use crate::error::{EditorError, EditorResult};
use crate::history::CommandHistory;
use crate::types::{CreateResult, MutationResult, RenderNode, SplitResult, UndoRedoResult};
use crate::{mutation, projection};

/// The editor service holds a database connection and per-landmark undo histories.
///
/// Constructed when a territory opens; dropped and recreated on territory switch.
pub struct EditorService {
    db: DatabaseConnection,
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
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            histories: HashMap::new(),
        }
    }

    // ----- Read operations -----

    /// Fetch a single mark by ID.
    pub async fn get_mark(&self, mark_id: MarkID) -> EditorResult<Mark> {
        store::query::mark::get(&self.db, mark_id)
            .await?
            .ok_or(EditorError::NotFound(mark_id.inner()))
    }

    /// List all landmark marks (equivalent to old `mark_list`).
    pub async fn mark_list(&self) -> EditorResult<Vec<Mark>> {
        Ok(store::query::mark::list_landmarks(&self.db).await?)
    }

    /// Project a landmark into a flat block list.
    pub async fn project_landmark(
        &self,
        landmark_id: MarkID,
        max_depth: Option<u32>,
    ) -> EditorResult<Vec<RenderNode>> {
        projection::project_landmark(&self.db, landmark_id, max_depth).await
    }

    // ----- Ported write operations -----

    pub async fn create_mark(
        &mut self,
        id: MarkID,
        parent_id: MarkID,
        content: Content,
        created_at: &str,
        after_id: Option<MarkID>,
        landmark_id: MarkID,
    ) -> EditorResult<CreateResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::create_mark(
            &self.db,
            h,
            id,
            parent_id,
            content,
            created_at,
            after_id,
            landmark_id,
        )
        .await
    }

    pub async fn update_mark(
        &mut self,
        id: MarkID,
        content: Content,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::update_mark(&self.db, h, id, content, landmark_id).await
    }

    pub async fn delete_mark(
        &mut self,
        id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::delete_mark(&self.db, h, id, landmark_id).await
    }

    pub async fn move_mark(
        &mut self,
        parent_id: MarkID,
        child_id: MarkID,
        after_id: Option<MarkID>,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::move_mark(&self.db, h, parent_id, child_id, after_id, landmark_id).await
    }

    pub async fn update_mark_title(&self, id: MarkID, title: RichText) -> EditorResult<()> {
        mutation::update_mark_title(&self.db, id, title).await
    }

    // ----- New structural operations -----

    pub async fn create_mark_after(
        &mut self,
        after_id: MarkID,
        parent_id: MarkID,
        data: MarkData,
        landmark_id: MarkID,
    ) -> EditorResult<CreateResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::create_mark_after(&self.db, h, after_id, parent_id, data, landmark_id).await
    }

    pub async fn update_mark_text(
        &mut self,
        mark_id: MarkID,
        rich_text: RichText,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::update_mark_text(&self.db, h, mark_id, rich_text, landmark_id).await
    }

    pub async fn split_mark(
        &mut self,
        mark_id: MarkID,
        offset: u32,
        landmark_id: MarkID,
    ) -> EditorResult<SplitResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::split_mark(&self.db, h, mark_id, offset as usize, landmark_id).await
    }

    pub async fn merge_marks(
        &mut self,
        target_id: MarkID,
        source_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::merge_marks(&self.db, h, target_id, source_id, landmark_id).await
    }

    pub async fn delete_marks(
        &mut self,
        mark_ids: &[MarkID],
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::delete_marks(&self.db, h, mark_ids, landmark_id).await
    }

    pub async fn reparent_mark(
        &mut self,
        mark_id: MarkID,
        new_parent_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::reparent_mark(&self.db, h, mark_id, new_parent_id, landmark_id).await
    }

    pub async fn swap_mark_order(
        &mut self,
        mark_id_a: MarkID,
        mark_id_b: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::swap_mark_order(&self.db, h, mark_id_a, mark_id_b, landmark_id).await
    }

    // ----- Undo/Redo -----

    pub async fn undo(&mut self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        let h = history_entry(&mut self.histories, landmark_id);
        let cursor = h.undo(&self.db).await?;
        match cursor {
            Some(c) => {
                let blocks = projection::project_landmark(&self.db, landmark_id, None).await?;
                Ok(Some(UndoRedoResult {
                    blocks,
                    cursor: Some(c),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn redo(&mut self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        let h = history_entry(&mut self.histories, landmark_id);
        let cursor = h.redo(&self.db).await?;
        match cursor {
            Some(c) => {
                let blocks = projection::project_landmark(&self.db, landmark_id, None).await?;
                Ok(Some(UndoRedoResult {
                    blocks,
                    cursor: Some(c),
                }))
            }
            None => Ok(None),
        }
    }
}
