use crate::{
    CommandHistory, CreateResult, Database, EditorError, EditorResult, MutationResult, RenderNode,
    SplitResult, UndoRedoResult,
    editor::{mutation, projection},
};
use domain::{Content, Mark, MarkData, MarkID, RichText};
use std::collections::HashMap;
use store::Store;

/// The editor service holds a shared store handle and per-landmark undo
/// histories.
///
/// The [`Database`](crate::Database) Arc is shared with `TerritoryManager` so territory switches
/// only need to swap the inner `Option` — no EditorService reconstruction required.
pub struct EditorService {
    db: Database,
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
    pub fn new(db: Database) -> Self {
        Self {
            db,
            histories: HashMap::new(),
        }
    }

    /// Lock the shared Arc, clone the store handle (cheap Arc clone), release lock.
    async fn db(&self) -> EditorResult<Store> {
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

    pub async fn update_mark_title(&mut self, id: MarkID, title: RichText) -> EditorResult<()> {
        tracing::info!(%id, "mark_update_title");
        let db = self.db().await?;
        // A title edit's history bucket is the landmark's own ID — the page
        // whose history the user expects Cmd+Z to walk while editing it.
        let h = history_entry(&mut self.histories, id);
        mutation::update_mark_title(&db, h, id, title).await
    }

    pub async fn update_mark_reading(
        &mut self,
        id: MarkID,
        feature_id: domain::FeatureID,
        reading: domain::Reading,
    ) -> EditorResult<Mark> {
        let db = self.db().await?;
        // A reading edit's history bucket is the landmark's own ID, same as
        // title edits.
        let h = history_entry(&mut self.histories, id);
        mutation::update_mark_reading(&db, h, id, feature_id, reading).await
    }

    pub async fn create_landmark(&mut self, chart_id: domain::ChartID) -> EditorResult<Mark> {
        let db = self.db().await?;
        // The mutation mints the new MarkID and keys its history bucket with
        // it, so it takes the whole histories map rather than one bucket.
        let created = mutation::create_landmark(&db, &mut self.histories, chart_id).await?;
        tracing::info!(id = %created.id, type_name = created.type_name(), "landmark_create: done");
        Ok(created)
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

    pub async fn wrap_in_compound(
        &mut self,
        compound_data: MarkData,
        child_ids: &[MarkID],
        after_id: Option<MarkID>,
        parent_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<CreateResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::wrap_in_compound(
            &db,
            h,
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
        after_id: Option<MarkID>,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        mutation::reparent_mark(&db, h, mark_id, new_parent_id, after_id, landmark_id).await
    }

    pub async fn undo(&mut self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        let db = self.db().await?;
        let h = history_entry(&mut self.histories, landmark_id);
        match h.undo(&db).await? {
            Some((cursor, _ops)) => {
                let blocks = project_after_history_walk(&db, landmark_id).await?;
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
        let h = history_entry(&mut self.histories, landmark_id);
        match h.redo(&db).await? {
            Some((cursor, _ops)) => {
                let blocks = project_after_history_walk(&db, landmark_id).await?;
                Ok(Some(UndoRedoResult {
                    blocks,
                    cursor: Some(cursor),
                }))
            }
            None => Ok(None),
        }
    }
}

/// Project a landmark after an undo/redo step. The step itself may remove the
/// landmark — undoing its own `CreateMark` soft-deletes it — in which case
/// there is nothing left to project and the walk still succeeded.
async fn project_after_history_walk(
    db: &Store,
    landmark_id: MarkID,
) -> EditorResult<Vec<RenderNode>> {
    match projection::project_landmark(db, landmark_id, None).await {
        Ok(blocks) => Ok(blocks),
        Err(EditorError::NotFound(_)) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}
