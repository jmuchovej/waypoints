use domain::chart::FeatureID;
use domain::fractional_index;
use domain::mark::{Content, Mark, MarkData, MarkID, Reading, RichText};
use store::DatabaseConnection;
use store::query::{mark, terrain};

use crate::error::{EditorError, EditorResult};
use crate::history::CommandHistory;
use crate::ops::AtomicOp;
use crate::projection;
use crate::types::{CreateResult, CursorState, MutationResult, RenderNode, SplitResult};

/// Compute a fractional index for inserting after `after_id` among `parent_id`'s children.
async fn compute_order(
    db: &DatabaseConnection,
    parent_id: MarkID,
    after_id: Option<MarkID>,
) -> EditorResult<String> {
    let edges = terrain::get_child_edges(db, parent_id).await?;

    let new_order = if let Some(after) = after_id {
        let after_idx = edges.iter().position(|e| e.dst_id == after);
        match after_idx {
            Some(idx) => {
                let after_order = edges[idx].order.as_deref();
                let next_order = edges.get(idx + 1).and_then(|e| e.order.as_deref());
                fractional_index::key_between(after_order, next_order)
            }
            None => {
                fractional_index::key_between(edges.last().and_then(|e| e.order.as_deref()), None)
            }
        }
    } else if let Some(first) = edges.first() {
        fractional_index::key_between(None, first.order.as_deref())
    } else {
        fractional_index::key_between(None, None)
    };
    Ok(new_order)
}

/// Compute a fractional index for inserting, skipping `exclude_id` in the ordering.
async fn compute_order_excluding(
    db: &DatabaseConnection,
    parent_id: MarkID,
    after_id: Option<MarkID>,
    exclude_id: MarkID,
) -> EditorResult<String> {
    let edges = terrain::get_child_edges(db, parent_id).await?;

    let new_order = if let Some(after) = after_id {
        let after_idx = edges.iter().position(|e| e.dst_id == after);
        match after_idx {
            Some(idx) => {
                let after_order = edges[idx].order.as_deref();
                let next_order = edges[idx + 1..]
                    .iter()
                    .find(|e| e.dst_id != exclude_id)
                    .and_then(|e| e.order.as_deref());
                fractional_index::key_between(after_order, next_order)
            }
            None => {
                fractional_index::key_between(edges.last().and_then(|e| e.order.as_deref()), None)
            }
        }
    } else {
        let first_order = edges
            .iter()
            .find(|e| e.dst_id != exclude_id)
            .and_then(|e| e.order.as_deref());
        fractional_index::key_between(None, first_order)
    };
    Ok(new_order)
}

/// Find the parent edge for a mark.
async fn find_parent_edge(
    db: &DatabaseConnection,
    mark_id: MarkID,
) -> EditorResult<(MarkID, String)> {
    let edge = terrain::find_parent_edge(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;
    Ok((edge.src_id, edge.order.unwrap_or_default()))
}

/// Update a mark's content (ported from `mark_update` command).
pub async fn update_mark(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    id: MarkID,
    content: Content,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let m = mark::get(db, id)
        .await?
        .ok_or(EditorError::NotFound(id.inner()))?;

    let old_data = m.data;
    let new_data = MarkData::Content(content);

    mark::update_data(db, id, &new_data).await?;

    let cursor = CursorState {
        block_id: id,
        offset: None,
    };
    history.push(
        vec![AtomicOp::UpdateContent {
            mark_id: id,
            old: old_data,
            new: new_data,
        }],
        id,
        cursor.clone(),
        cursor,
        false,
    );

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Delete a mark (ported from `mark_delete` command).
pub async fn delete_mark(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let m = mark::get(db, id)
        .await?
        .ok_or(EditorError::NotFound(id.inner()))?;

    let (parent_id, frac_idx) = find_parent_edge(db, id).await.ok().unzip();

    mark::soft_delete(db, id).await?;
    terrain::delete_edges_by_dst(db, id).await?;

    let cursor = CursorState {
        block_id: id,
        offset: None,
    };
    history.push(
        vec![AtomicOp::DeleteMark {
            mark_id: id,
            data: m.data,
            parent_id,
            fractional_index: frac_idx.unwrap_or_default(),
        }],
        id,
        cursor.clone(),
        cursor,
        false,
    );

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Move one or more marks to a new position (batch version of the old `move_mark`).
///
/// `child_ids` must be in visual order (top-to-bottom). Each block is placed
/// sequentially after the previous one, so `compute_order_excluding` always
/// sees the updated position of the preceding block.
pub async fn move_marks(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    parent_id: MarkID,
    child_ids: &[MarkID],
    after_id: Option<MarkID>,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let mut ops = Vec::new();
    let mut prev_id = after_id;

    for &child_id in child_ids {
        let (old_parent, old_index) = find_parent_edge(db, child_id).await?;
        let new_order = compute_order_excluding(db, parent_id, prev_id, child_id).await?;
        terrain::update_child_edge(db, child_id, Some(parent_id), &new_order).await?;

        ops.push(AtomicOp::Reparent {
            mark_id: child_id,
            old_parent,
            new_parent: parent_id,
            old_index,
            new_index: new_order,
        });
        prev_id = Some(child_id);
    }

    if let Some(&first) = child_ids.first() {
        let cursor = CursorState {
            block_id: first,
            offset: None,
        };
        history.push(ops, first, cursor.clone(), cursor, false);
    }

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Update a landmark's title (ported from `mark_update_title` command).
pub async fn update_mark_title(
    db: &DatabaseConnection,
    id: MarkID,
    title: RichText,
) -> EditorResult<()> {
    let m = mark::get(db, id)
        .await?
        .ok_or(EditorError::NotFound(id.inner()))?;

    let mut mark_data = m.data;
    match &mut mark_data {
        MarkData::Landmark(landmark) => {
            landmark.title = title;
        }
        _ => return Err(EditorError::NotLandmark(id.inner())),
    }

    mark::update_data(db, id, &mark_data).await?;
    Ok(())
}

/// Set or remove a reading on a landmark. If `reading` is `Reading::Null`, the entry is removed.
pub async fn update_mark_reading(
    db: &DatabaseConnection,
    id: MarkID,
    feature_id: FeatureID,
    reading: Reading,
) -> EditorResult<Mark> {
    let m = mark::get(db, id)
        .await?
        .ok_or(EditorError::NotFound(id.inner()))?;

    let mut mark_data = m.data;
    match &mut mark_data {
        MarkData::Landmark(landmark) => {
            if matches!(reading, Reading::Null) {
                landmark.readings.shift_remove(&feature_id);
            } else {
                landmark.readings.insert(feature_id, reading);
            }
        }
        _ => return Err(EditorError::NotLandmark(id.inner())),
    }

    mark::update_data(db, id, &mark_data).await?;

    // Re-fetch and return the updated mark
    let updated = mark::get(db, id)
        .await?
        .ok_or(EditorError::NotFound(id.inner()))?;
    Ok(updated)
}

/// Create a mark after a specified sibling.
pub async fn create_mark_after(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    after_id: MarkID,
    parent_id: MarkID,
    data: MarkData,
    landmark_id: MarkID,
) -> EditorResult<CreateResult> {
    let new_id = MarkID::new();
    let new_order = compute_order(db, parent_id, Some(after_id)).await?;
    let now = chrono::Local::now().naive_utc();

    mark::insert(db, new_id, &data, now).await?;
    terrain::insert_child_edge(db, parent_id, new_id, &new_order).await?;

    let cursor = CursorState {
        block_id: new_id,
        offset: Some(0),
    };
    history.push(
        vec![AtomicOp::CreateMark {
            mark_id: new_id,
            data,
            parent_id: Some(parent_id),
            fractional_index: new_order,
        }],
        new_id,
        cursor.clone(),
        cursor,
        false,
    );

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(CreateResult {
        new_mark_id: new_id,
        blocks,
    })
}

/// Update just the RichText content of a mark (records as text edit for coalescing).
pub async fn update_mark_text(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    mark_id: MarkID,
    rich_text: RichText,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let m = mark::get(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;

    let mut mark_data = m.data;
    let old_rt = match &mark_data {
        MarkData::Content(c) => c
            .rich_text()
            .cloned()
            .ok_or_else(|| EditorError::InvalidContent("content has no rich_text".into()))?,
        _ => return Err(EditorError::InvalidContent("not a content mark".into())),
    };

    if let MarkData::Content(c) = &mut mark_data {
        c.set_rich_text(rich_text.clone());
    }

    mark::update_data(db, mark_id, &mark_data).await?;

    let cursor = CursorState {
        block_id: mark_id,
        offset: None,
    };
    history.push(
        vec![AtomicOp::UpdateRichText {
            mark_id,
            old: old_rt,
            new: rich_text,
        }],
        mark_id,
        cursor.clone(),
        cursor,
        true, // text edit — eligible for coalescing
    );

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Split a mark at an offset: the original keeps content before the offset,
/// a new sibling is created with content after.
pub async fn split_mark(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    mark_id: MarkID,
    offset: usize,
    landmark_id: MarkID,
) -> EditorResult<SplitResult> {
    let m = mark::get(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;

    let old_content = match &m.data {
        MarkData::Content(c) => c.clone(),
        _ => {
            return Err(EditorError::InvalidContent(
                "cannot split a landmark".into(),
            ));
        }
    };

    let old_rt = old_content
        .rich_text()
        .cloned()
        .ok_or_else(|| EditorError::InvalidContent("content has no rich_text field".into()))?;

    let (left_rt, right_rt) = crate::rich_text::split_at(&old_rt, offset);

    // Update original mark with left portion
    let mut left_content = old_content.clone();
    left_content.set_rich_text(left_rt.clone());
    let left_data = MarkData::Content(left_content);
    mark::update_data(db, mark_id, &left_data).await?;

    // Create new mark with right portion
    let new_id = MarkID::new();
    let mut right_content = old_content;
    right_content.set_rich_text(right_rt);
    let right_data = MarkData::Content(right_content);

    let (parent_id, _) = find_parent_edge(db, mark_id).await?;
    let new_order = compute_order(db, parent_id, Some(mark_id)).await?;
    let now = chrono::Local::now().naive_utc();

    mark::insert(db, new_id, &right_data, now).await?;
    terrain::insert_child_edge(db, parent_id, new_id, &new_order).await?;

    // Record ops
    let cursor_before = CursorState {
        block_id: mark_id,
        offset: Some(offset as u32),
    };
    let cursor_after = CursorState {
        block_id: new_id,
        offset: Some(0),
    };
    history.push(
        vec![
            AtomicOp::UpdateRichText {
                mark_id,
                old: old_rt,
                new: left_rt,
            },
            AtomicOp::CreateMark {
                mark_id: new_id,
                data: right_data,
                parent_id: Some(parent_id),
                fractional_index: new_order,
            },
        ],
        mark_id,
        cursor_before,
        cursor_after,
        false,
    );

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(SplitResult {
        new_mark_id: new_id,
        blocks,
    })
}

/// Merge source mark's RichText into target, then delete source.
pub async fn merge_marks(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    target_id: MarkID,
    source_id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    // Fetch target
    let target = mark::get(db, target_id)
        .await?
        .ok_or(EditorError::NotFound(target_id.inner()))?;

    let old_target_rt = match &target.data {
        MarkData::Content(c) => c
            .rich_text()
            .cloned()
            .ok_or_else(|| EditorError::InvalidContent("target has no rich_text".into()))?,
        _ => return Err(EditorError::InvalidContent("target is not content".into())),
    };

    // Fetch source
    let source = mark::get(db, source_id)
        .await?
        .ok_or(EditorError::NotFound(source_id.inner()))?;

    let source_rt = match &source.data {
        MarkData::Content(c) => c
            .rich_text()
            .cloned()
            .ok_or_else(|| EditorError::InvalidContent("source has no rich_text".into()))?,
        _ => return Err(EditorError::InvalidContent("source is not content".into())),
    };

    // Merge: append source text to target
    let merge_offset = old_target_rt.plain_text().len();
    let new_target_rt = crate::rich_text::merge(&old_target_rt, &source_rt);

    // Update target
    let mut new_target_data = target.data;
    if let MarkData::Content(c) = &mut new_target_data {
        c.set_rich_text(new_target_rt.clone());
    }
    mark::update_data(db, target_id, &new_target_data).await?;

    // Delete source
    let (source_parent, source_idx) = find_parent_edge(db, source_id).await?;
    mark::soft_delete(db, source_id).await?;
    terrain::delete_edges_by_dst(db, source_id).await?;

    // Record ops
    let cursor_before = CursorState {
        block_id: source_id,
        offset: Some(0),
    };
    let cursor_after = CursorState {
        block_id: target_id,
        offset: Some(merge_offset as u32),
    };
    history.push(
        vec![
            AtomicOp::UpdateRichText {
                mark_id: target_id,
                old: old_target_rt,
                new: new_target_rt,
            },
            AtomicOp::DeleteMark {
                mark_id: source_id,
                data: source.data,
                parent_id: Some(source_parent),
                fractional_index: source_idx,
            },
        ],
        target_id,
        cursor_before,
        cursor_after,
        false,
    );

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Batch soft-delete multiple marks.
pub async fn delete_marks(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    mark_ids: &[MarkID],
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let mut ops = Vec::new();

    for &mid in mark_ids {
        let m = mark::get(db, mid)
            .await?
            .ok_or(EditorError::NotFound(mid.inner()))?;

        let (parent_id, frac_idx) = find_parent_edge(db, mid).await.ok().unzip();

        mark::soft_delete(db, mid).await?;
        terrain::delete_edges_by_dst(db, mid).await?;

        ops.push(AtomicOp::DeleteMark {
            mark_id: mid,
            data: m.data,
            parent_id,
            fractional_index: frac_idx.unwrap_or_default(),
        });
    }

    if let Some(&first) = mark_ids.first() {
        let cursor = CursorState {
            block_id: first,
            offset: None,
        };
        history.push(ops, first, cursor.clone(), cursor, false);
    }

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Reparent a mark (indent/dedent).
pub async fn reparent_mark(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    mark_id: MarkID,
    new_parent_id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let (old_parent, old_index) = find_parent_edge(db, mark_id).await?;

    // Append at end of new parent's children
    let edges = terrain::get_child_edges(db, new_parent_id).await?;
    let new_order =
        fractional_index::key_between(edges.last().and_then(|e| e.order.as_deref()), None);

    terrain::update_child_edge(db, mark_id, Some(new_parent_id), &new_order).await?;

    let cursor = CursorState {
        block_id: mark_id,
        offset: None,
    };
    history.push(
        vec![AtomicOp::Reparent {
            mark_id,
            old_parent,
            new_parent: new_parent_id,
            old_index,
            new_index: new_order.into(),
        }],
        mark_id,
        cursor.clone(),
        cursor,
        false,
    );

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}
