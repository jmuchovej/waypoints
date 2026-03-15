use domain::mark::{MarkData, MarkID, RichText};
use store::DatabaseConnection;
use store::query::{mark, terrain};

use crate::error::{EditorError, EditorResult};

/// An atomic, invertible operation on a single mark or terrain edge.
#[derive(Debug, Clone)]
pub enum AtomicOp {
    UpdateRichText {
        mark_id: MarkID,
        old: RichText,
        new: RichText,
    },
    UpdateContent {
        mark_id: MarkID,
        old: MarkData,
        new: MarkData,
    },
    CreateMark {
        mark_id: MarkID,
        data: MarkData,
        parent_id: Option<MarkID>,
        fractional_index: String,
    },
    DeleteMark {
        mark_id: MarkID,
        data: MarkData,
        parent_id: Option<MarkID>,
        fractional_index: String,
    },
    Reparent {
        mark_id: MarkID,
        old_parent: MarkID,
        new_parent: MarkID,
        old_index: String,
        new_index: String,
    },
}

impl AtomicOp {
    /// Return the inverse operation that undoes this one.
    pub fn invert(&self) -> AtomicOp {
        match self {
            AtomicOp::UpdateRichText { mark_id, old, new } => AtomicOp::UpdateRichText {
                mark_id: *mark_id,
                old: new.clone(),
                new: old.clone(),
            },
            AtomicOp::UpdateContent { mark_id, old, new } => AtomicOp::UpdateContent {
                mark_id: *mark_id,
                old: new.clone(),
                new: old.clone(),
            },
            AtomicOp::CreateMark {
                mark_id,
                data,
                parent_id,
                fractional_index,
            } => AtomicOp::DeleteMark {
                mark_id: *mark_id,
                data: data.clone(),
                parent_id: *parent_id,
                fractional_index: fractional_index.clone(),
            },
            AtomicOp::DeleteMark {
                mark_id,
                data,
                parent_id,
                fractional_index,
            } => AtomicOp::CreateMark {
                mark_id: *mark_id,
                data: data.clone(),
                parent_id: *parent_id,
                fractional_index: fractional_index.clone(),
            },
            AtomicOp::Reparent {
                mark_id,
                old_parent,
                new_parent,
                old_index,
                new_index,
            } => AtomicOp::Reparent {
                mark_id: *mark_id,
                old_parent: *new_parent,
                new_parent: *old_parent,
                old_index: new_index.clone(),
                new_index: old_index.clone(),
            },
        }
    }

    /// Execute this operation against the database.
    pub async fn apply(&self, db: &DatabaseConnection) -> EditorResult<()> {
        match self {
            AtomicOp::UpdateRichText { mark_id, new, .. } => {
                apply_rich_text_update(db, *mark_id, new).await
            }
            AtomicOp::UpdateContent { mark_id, new, .. } => {
                apply_content_update(db, *mark_id, new).await
            }
            AtomicOp::CreateMark {
                mark_id,
                data,
                parent_id,
                fractional_index,
            } => apply_create_mark(db, *mark_id, data, *parent_id, fractional_index).await,
            AtomicOp::DeleteMark { mark_id, .. } => apply_delete_mark(db, *mark_id).await,
            AtomicOp::Reparent {
                mark_id,
                new_parent,
                new_index,
                ..
            } => apply_reparent(db, *mark_id, *new_parent, new_index).await,
        }
    }
}

/// Update the RichText content of a mark (for Content variants that have RichText).
async fn apply_rich_text_update(
    db: &DatabaseConnection,
    mark_id: MarkID,
    new_rt: &RichText,
) -> EditorResult<()> {
    let m = mark::get(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;

    let mut mark_data = m.data;
    match &mut mark_data {
        MarkData::Content(content) => {
            if !content.set_rich_text(new_rt.clone()) {
                return Err(EditorError::InvalidContent(
                    "content variant has no rich_text field".into(),
                ));
            }
        }
        _ => {
            return Err(EditorError::InvalidContent(
                "cannot update RichText on a Landmark".into(),
            ));
        }
    }

    mark::update_data(db, mark_id, &mark_data).await?;
    Ok(())
}

/// Update the full MarkData of a mark.
async fn apply_content_update(
    db: &DatabaseConnection,
    mark_id: MarkID,
    new_data: &MarkData,
) -> EditorResult<()> {
    // Verify the mark exists
    mark::get(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;

    mark::update_data(db, mark_id, new_data).await?;
    Ok(())
}

/// Create a new mark and its terrain edge.
/// If the mark already exists (e.g. undoing a soft-delete), restore it instead.
async fn apply_create_mark(
    db: &DatabaseConnection,
    mark_id: MarkID,
    data: &MarkData,
    parent_id: Option<MarkID>,
    fractional_index: &str,
) -> EditorResult<()> {
    match mark::restore(db, mark_id).await {
        Ok(()) => {
            // Restored a soft-deleted mark — update its data to match
            mark::update_data(db, mark_id, data).await?;
        }
        Err(_) => {
            // Mark doesn't exist at all — fresh insert
            let now = chrono::Local::now().naive_utc();
            mark::insert(db, mark_id, data, now).await?;
        }
    }

    if let Some(pid) = parent_id {
        terrain::insert_child_edge(db, pid, mark_id, fractional_index).await?;
    }
    Ok(())
}

/// Soft-delete a mark and remove its terrain edges.
async fn apply_delete_mark(db: &DatabaseConnection, mark_id: MarkID) -> EditorResult<()> {
    mark::soft_delete(db, mark_id).await?;
    terrain::delete_edges_by_dst(db, mark_id).await?;
    Ok(())
}

/// Move a mark to a new parent with a new fractional index.
async fn apply_reparent(
    db: &DatabaseConnection,
    mark_id: MarkID,
    new_parent: MarkID,
    new_index: &str,
) -> EditorResult<()> {
    terrain::update_child_edge(db, mark_id, Some(new_parent), new_index).await?;
    Ok(())
}
