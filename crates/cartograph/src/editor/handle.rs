use crate::{
    CreateResult, EditorResult, EditorService, MutationResult, RenderNode, SplitResult,
    UndoRedoResult,
};
use domain::{Content, FeatureID, Mark, MarkData, MarkID, Reading, RichText};
use tokio::sync::Mutex;

pub struct EditorHandle<'a> {
    pub(crate) editor: &'a Mutex<EditorService>,
}

impl EditorHandle<'_> {
    pub async fn get_mark(&self, mark_id: MarkID) -> EditorResult<Mark> {
        self.editor.lock().await.get_mark(mark_id).await
    }

    pub async fn mark_list(&self) -> EditorResult<Vec<Mark>> {
        self.editor.lock().await.mark_list().await
    }

    pub async fn project_landmark(
        &self,
        landmark_id: MarkID,
        max_depth: Option<u32>,
    ) -> EditorResult<Vec<RenderNode>> {
        self.editor
            .lock()
            .await
            .project_landmark(landmark_id, max_depth)
            .await
    }

    pub async fn update_mark(
        &self,
        id: MarkID,
        content: Content,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        self.editor
            .lock()
            .await
            .update_mark(id, content, landmark_id)
            .await
    }

    pub async fn delete_mark(
        &self,
        id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        self.editor.lock().await.delete_mark(id, landmark_id).await
    }

    pub async fn move_marks(
        &self,
        parent_id: MarkID,
        child_ids: &[MarkID],
        after_id: Option<MarkID>,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        self.editor
            .lock()
            .await
            .move_marks(parent_id, child_ids, after_id, landmark_id)
            .await
    }

    pub async fn update_mark_title(&self, id: MarkID, title: RichText) -> EditorResult<()> {
        self.editor.lock().await.update_mark_title(id, title).await
    }

    pub async fn update_mark_reading(
        &self,
        id: MarkID,
        feature_id: FeatureID,
        reading: Reading,
    ) -> EditorResult<Mark> {
        self.editor
            .lock()
            .await
            .update_mark_reading(id, feature_id, reading)
            .await
    }

    pub async fn create_landmark(&self, chart_id: domain::ChartID) -> EditorResult<Mark> {
        self.editor.lock().await.create_landmark(chart_id).await
    }

    pub async fn create_mark_after(
        &self,
        after_id: MarkID,
        parent_id: MarkID,
        data: MarkData,
        landmark_id: MarkID,
    ) -> EditorResult<CreateResult> {
        self.editor
            .lock()
            .await
            .create_mark_after(after_id, parent_id, data, landmark_id)
            .await
    }

    pub async fn wrap_in_compound(
        &self,
        compound_data: MarkData,
        child_ids: &[MarkID],
        after_id: Option<MarkID>,
        parent_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<CreateResult> {
        self.editor
            .lock()
            .await
            .wrap_in_compound(compound_data, child_ids, after_id, parent_id, landmark_id)
            .await
    }

    pub async fn update_mark_text(
        &self,
        mark_id: MarkID,
        rich_text: RichText,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        self.editor
            .lock()
            .await
            .update_mark_text(mark_id, rich_text, landmark_id)
            .await
    }

    pub async fn split_mark(
        &self,
        mark_id: MarkID,
        offset: u32,
        landmark_id: MarkID,
    ) -> EditorResult<SplitResult> {
        self.editor
            .lock()
            .await
            .split_mark(mark_id, offset, landmark_id)
            .await
    }

    pub async fn merge_marks(
        &self,
        target_id: MarkID,
        source_id: MarkID,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        self.editor
            .lock()
            .await
            .merge_marks(target_id, source_id, landmark_id)
            .await
    }

    pub async fn delete_marks(
        &self,
        mark_ids: &[MarkID],
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        self.editor
            .lock()
            .await
            .delete_marks(mark_ids, landmark_id)
            .await
    }

    pub async fn reparent_mark(
        &self,
        mark_id: MarkID,
        new_parent_id: MarkID,
        after_id: Option<MarkID>,
        landmark_id: MarkID,
    ) -> EditorResult<MutationResult> {
        self.editor
            .lock()
            .await
            .reparent_mark(mark_id, new_parent_id, after_id, landmark_id)
            .await
    }

    pub async fn undo(&self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        self.editor.lock().await.undo(landmark_id).await
    }

    pub async fn redo(&self, landmark_id: MarkID) -> EditorResult<Option<UndoRedoResult>> {
        self.editor.lock().await.redo(landmark_id).await
    }
}
