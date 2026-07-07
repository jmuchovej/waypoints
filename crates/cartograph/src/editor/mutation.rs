use domain::chart::{FeatureID, FeatureKind};
use domain::fractional_index;
use domain::mark::{Content, Mark, MarkData, MarkID, Reading, RichText};
use domain::terrain::{Terrain, TerrainKind};
use store::DatabaseConnection;
use store::query::{chart, mark, terrain};

use crate::sync::{SyncCoordinator, TextPayload, touched_marks};

use super::error::{EditorError, EditorResult};
use super::history::CommandHistory;
use super::maneuver::Maneuver;
use super::projection;
use super::types::{CreateResult, CursorState, MutationResult, RenderNode, SplitResult};

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
async fn find_parent_edge(db: &DatabaseConnection, mark_id: MarkID) -> EditorResult<Terrain> {
    terrain::find_parent_edge(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))
}

/// Snapshot an edge into a `DeleteTerrain` op (full snapshot for invertibility).
fn delete_edge_op(edge: Terrain) -> Maneuver {
    Maneuver::DeleteTerrain {
        edge_id: edge.id,
        src: edge.src_id,
        dst: edge.dst_id,
        kind: edge.kind,
        order: edge.order,
        extra: edge.extra,
    }
}

/// Update a mark's content (ported from `mark_update` command).
pub async fn update_mark(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    sync: &mut SyncCoordinator,
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
    let ops = vec![Maneuver::UpdateMarkData {
        mark_id: id,
        old: old_data,
        new: new_data,
    }];
    history.push(ops.clone(), id, cursor.clone(), cursor, false);
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Delete a mark (ported from `mark_delete` command).
pub async fn delete_mark(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    sync: &mut SyncCoordinator,
    id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    mark::get(db, id)
        .await?
        .ok_or(EditorError::NotFound(id.inner()))?;

    // Snapshot incoming edges before deleting them so undo can recreate
    // each with its original TerrainID.
    let edges = terrain::get_edges(db, None, Some(id), None).await?;

    mark::soft_delete(db, id).await?;
    terrain::delete_edges_by_dst(db, id, None).await?;

    let mut ops = vec![Maneuver::SoftDeleteMark {
        mark_id: id,
        deleted_at: chrono::Local::now().naive_utc(),
    }];
    ops.extend(edges.into_iter().map(delete_edge_op));

    let cursor = CursorState {
        block_id: id,
        offset: None,
    };
    history.push(ops.clone(), id, cursor.clone(), cursor, false);
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

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
    sync: &mut SyncCoordinator,
    parent_id: MarkID,
    child_ids: &[MarkID],
    after_id: Option<MarkID>,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let mut ops = Vec::new();
    let mut prev_id = after_id;

    for &child_id in child_ids {
        let edge = find_parent_edge(db, child_id).await?;
        let new_order = compute_order_excluding(db, parent_id, prev_id, child_id).await?;
        terrain::update_edge_by_id(db, edge.id, parent_id, Some(&new_order)).await?;

        ops.push(Maneuver::UpdateTerrain {
            edge_id: edge.id,
            old_src: edge.src_id,
            old_order: edge.order,
            new_src: parent_id,
            new_order: Some(new_order),
        });
        prev_id = Some(child_id);
    }

    if let Some(&first) = child_ids.first() {
        let cursor = CursorState {
            block_id: first,
            offset: None,
        };
        history.push(ops.clone(), first, cursor.clone(), cursor, false);
        let marks = touched_marks(&ops);
        sync.log_local(db, ops, marks).await?;
    }

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Update a landmark's title (ported from `mark_update_title` command).
pub async fn update_mark_title(
    db: &DatabaseConnection,
    sync: &mut SyncCoordinator,
    id: MarkID,
    title: RichText,
) -> EditorResult<()> {
    let m = mark::get(db, id)
        .await?
        .ok_or(EditorError::NotFound(id.inner()))?;

    let old_data = m.data.clone();
    let mut mark_data = m.data;
    match &mut mark_data {
        MarkData::Landmark(landmark) => {
            landmark.title = title;
        }
        _ => return Err(EditorError::NotLandmark(id.inner())),
    }

    mark::update_data(db, id, &mark_data).await?;

    // Not undoable (no history push), but still a synced write.
    let ops = vec![Maneuver::UpdateMarkData {
        mark_id: id,
        old: old_data,
        new: mark_data,
    }];
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;
    Ok(())
}

/// Set or remove a reading on a landmark. If `reading` is `Reading::Null`, the entry is removed.
pub async fn update_mark_reading(
    db: &DatabaseConnection,
    sync: &mut SyncCoordinator,
    id: MarkID,
    feature_id: FeatureID,
    reading: Reading,
) -> EditorResult<Mark> {
    let m = mark::get(db, id)
        .await?
        .ok_or(EditorError::NotFound(id.inner()))?;

    let old_data = m.data.clone();
    let mut mark_data = m.data;
    let chart_id = match &mark_data {
        MarkData::Landmark(l) => l.chart.clone(),
        _ => return Err(EditorError::NotLandmark(id.inner())),
    };

    // Resolve the chart to validate the reading against the feature schema
    let chart = chart::get(db, chart_id.clone())
        .await?
        .ok_or_else(|| EditorError::Validation(format!("chart not found: {chart_id}")))?;

    let feature = chart
        .features
        .get(&feature_id)
        .ok_or_else(|| EditorError::Validation(format!("feature not found: {feature_id}")))?;

    // Run domain validation (max_references, type shape, etc.)
    feature
        .kind
        .validate_reading(&reading)
        .map_err(EditorError::Validation)?;

    // Validate allowed_types for reference readings
    if let (
        FeatureKind::Reference {
            allowed_types: Some(types),
            ..
        },
        Reading::References(refs),
    ) = (&feature.kind, &reading)
    {
        let ref_ids: Vec<uuid::Uuid> = refs.iter().map(|r| r.inner()).collect();
        let ref_marks = mark::get_many(db, &ref_ids).await?;

        for ref_mark in &ref_marks {
            match &ref_mark.data {
                MarkData::Landmark(l) => {
                    if !types.contains(&l.chart) {
                        return Err(EditorError::Validation(format!(
                            "reference to {} not allowed (chart {} not in allowed_types)",
                            ref_mark.id, l.chart
                        )));
                    }
                }
                _ => {
                    return Err(EditorError::Validation(format!(
                        "reference {} is not a landmark",
                        ref_mark.id
                    )));
                }
            }
        }
    }

    match &mut mark_data {
        MarkData::Landmark(landmark) => {
            if matches!(reading, Reading::Null) {
                landmark.readings.shift_remove(&feature_id);
            } else {
                landmark.readings.insert(feature_id, reading);
            }
        }
        _ => unreachable!(),
    }

    mark::update_data(db, id, &mark_data).await?;

    // Not undoable (no history push), but still a synced write.
    let ops = vec![Maneuver::UpdateMarkData {
        mark_id: id,
        old: old_data,
        new: mark_data,
    }];
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

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
    sync: &mut SyncCoordinator,
    after_id: MarkID,
    parent_id: MarkID,
    data: MarkData,
    landmark_id: MarkID,
) -> EditorResult<CreateResult> {
    let new_id = MarkID::new();
    let new_order = compute_order(db, parent_id, Some(after_id)).await?;
    let now = chrono::Local::now().naive_utc();

    mark::insert(db, new_id, &data, now).await?;
    let edge_id = terrain::insert_child_edge(db, parent_id, new_id, &new_order).await?;

    let cursor = CursorState {
        block_id: new_id,
        offset: Some(0),
    };
    let ops = vec![
        Maneuver::CreateMark {
            mark_id: new_id,
            data,
        },
        Maneuver::CreateTerrain {
            edge_id,
            src: parent_id,
            dst: new_id,
            kind: TerrainKind::Child,
            order: Some(new_order),
            extra: None,
        },
    ];
    history.push(ops.clone(), new_id, cursor.clone(), cursor, false);
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(CreateResult {
        new_mark_id: new_id,
        blocks,
    })
}

/// Update just the RichText content of a mark. Persists to DB but does NOT
/// record in the undo history — text undo is handled by Tiptap's local History
/// extension on the frontend.
pub async fn update_mark_text(
    db: &DatabaseConnection,
    _history: &mut CommandHistory,
    sync: &mut SyncCoordinator,
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

    // Text undo is handled by Tiptap's local History extension, so this
    // never enters CommandHistory — but it's still a synced write.
    let ops = vec![Maneuver::UpdateRichText {
        mark_id,
        old: old_rt,
        new: TextPayload::Snapshot { rich_text },
    }];
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Split a mark at an offset: the original keeps content before the offset,
/// a new sibling is created with content after.
pub async fn split_mark(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    sync: &mut SyncCoordinator,
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

    let (left_rt, right_rt) = super::rich_text::split_at(&old_rt, offset);

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

    let parent_id = find_parent_edge(db, mark_id).await?.src_id;
    let new_order = compute_order(db, parent_id, Some(mark_id)).await?;
    let now = chrono::Local::now().naive_utc();

    mark::insert(db, new_id, &right_data, now).await?;
    let edge_id = terrain::insert_child_edge(db, parent_id, new_id, &new_order).await?;

    // Record ops
    let cursor_before = CursorState {
        block_id: mark_id,
        offset: Some(offset as u32),
    };
    let cursor_after = CursorState {
        block_id: new_id,
        offset: Some(0),
    };
    let ops = vec![
        Maneuver::UpdateRichText {
            mark_id,
            old: old_rt,
            new: TextPayload::Snapshot { rich_text: left_rt },
        },
        Maneuver::CreateMark {
            mark_id: new_id,
            data: right_data,
        },
        Maneuver::CreateTerrain {
            edge_id,
            src: parent_id,
            dst: new_id,
            kind: TerrainKind::Child,
            order: Some(new_order),
            extra: None,
        },
    ];
    history.push(ops.clone(), mark_id, cursor_before, cursor_after, false);
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

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
    sync: &mut SyncCoordinator,
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
    let new_target_rt = super::rich_text::merge(&old_target_rt, &source_rt);

    // Update target
    let mut new_target_data = target.data;
    if let MarkData::Content(c) = &mut new_target_data {
        c.set_rich_text(new_target_rt.clone());
    }
    mark::update_data(db, target_id, &new_target_data).await?;

    // Delete source — snapshot its edges first so undo can recreate them.
    let source_edges = terrain::get_edges(db, None, Some(source_id), None).await?;
    mark::soft_delete(db, source_id).await?;
    terrain::delete_edges_by_dst(db, source_id, None).await?;

    // Record ops
    let cursor_before = CursorState {
        block_id: source_id,
        offset: Some(0),
    };
    let cursor_after = CursorState {
        block_id: target_id,
        offset: Some(merge_offset as u32),
    };
    let mut ops = vec![
        Maneuver::UpdateRichText {
            mark_id: target_id,
            old: old_target_rt,
            new: TextPayload::Snapshot {
                rich_text: new_target_rt,
            },
        },
        Maneuver::SoftDeleteMark {
            mark_id: source_id,
            deleted_at: chrono::Local::now().naive_utc(),
        },
    ];
    ops.extend(source_edges.into_iter().map(delete_edge_op));
    history.push(ops.clone(), target_id, cursor_before, cursor_after, false);
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Create a compound block (blockquote, callout, etc.) and either:
/// - Reparent the given `child_ids` under it, or
/// - Create an empty paragraph child if `child_ids` is empty.
///
/// The compound block is placed at the position of the first child (or after
/// `after_id` if no children are given).
pub async fn wrap_in_compound(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    sync: &mut SyncCoordinator,
    compound_data: MarkData,
    child_ids: &[MarkID],
    after_id: Option<MarkID>,
    parent_id: MarkID,
    landmark_id: MarkID,
) -> EditorResult<CreateResult> {
    let compound_id = MarkID::new();
    let now = chrono::Local::now().naive_utc();
    let mut ops = Vec::new();

    // Determine where to place the compound in the parent's child list.
    let compound_order = if let Some(first_child) = child_ids.first() {
        // Place at the first child's current position
        find_parent_edge(db, *first_child)
            .await?
            .order
            .unwrap_or_default()
    } else if let Some(after) = after_id {
        compute_order(db, parent_id, Some(after)).await?
    } else {
        compute_order(db, parent_id, None).await?
    };

    // Create the compound block
    mark::insert(db, compound_id, &compound_data, now).await?;
    let compound_edge_id =
        terrain::insert_child_edge(db, parent_id, compound_id, &compound_order).await?;
    ops.push(Maneuver::CreateMark {
        mark_id: compound_id,
        data: compound_data,
    });
    ops.push(Maneuver::CreateTerrain {
        edge_id: compound_edge_id,
        src: parent_id,
        dst: compound_id,
        kind: TerrainKind::Child,
        order: Some(compound_order),
        extra: None,
    });

    let focus_id;

    if child_ids.is_empty() {
        // No children to wrap — create an empty paragraph inside the compound
        let para_id = MarkID::new();
        let para_data = MarkData::Content(Content::default());
        let para_order = fractional_index::key_between(None, None);

        mark::insert(db, para_id, &para_data, now).await?;
        let para_edge_id =
            terrain::insert_child_edge(db, compound_id, para_id, &para_order).await?;
        ops.push(Maneuver::CreateMark {
            mark_id: para_id,
            data: para_data,
        });
        ops.push(Maneuver::CreateTerrain {
            edge_id: para_edge_id,
            src: compound_id,
            dst: para_id,
            kind: TerrainKind::Child,
            order: Some(para_order),
            extra: None,
        });
        focus_id = para_id;
    } else {
        // Reparent each child under the compound
        let mut prev_id: Option<MarkID> = None;
        for &child_id in child_ids {
            let edge = find_parent_edge(db, child_id).await?;
            let new_order = compute_order_excluding(db, compound_id, prev_id, child_id).await?;

            terrain::update_edge_by_id(db, edge.id, compound_id, Some(&new_order)).await?;
            ops.push(Maneuver::UpdateTerrain {
                edge_id: edge.id,
                old_src: edge.src_id,
                old_order: edge.order,
                new_src: compound_id,
                new_order: Some(new_order),
            });
            prev_id = Some(child_id);
        }
        focus_id = child_ids[0];
    }

    let cursor = CursorState {
        block_id: focus_id,
        offset: Some(0),
    };
    history.push(ops.clone(), compound_id, cursor.clone(), cursor, false);
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(CreateResult {
        new_mark_id: compound_id,
        blocks,
    })
}

/// Batch soft-delete multiple marks.
pub async fn delete_marks(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    sync: &mut SyncCoordinator,
    mark_ids: &[MarkID],
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let mut ops = Vec::new();

    for &mid in mark_ids {
        mark::get(db, mid)
            .await?
            .ok_or(EditorError::NotFound(mid.inner()))?;

        let edges = terrain::get_edges(db, None, Some(mid), None).await?;

        mark::soft_delete(db, mid).await?;
        terrain::delete_edges_by_dst(db, mid, None).await?;

        ops.push(Maneuver::SoftDeleteMark {
            mark_id: mid,
            deleted_at: chrono::Local::now().naive_utc(),
        });
        ops.extend(edges.into_iter().map(delete_edge_op));
    }

    if let Some(&first) = mark_ids.first() {
        let cursor = CursorState {
            block_id: first,
            offset: None,
        };
        history.push(ops.clone(), first, cursor.clone(), cursor, false);
        let marks = touched_marks(&ops);
        sync.log_local(db, ops, marks).await?;
    }

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}

/// Reparent a mark (indent/dedent).
pub async fn reparent_mark(
    db: &DatabaseConnection,
    history: &mut CommandHistory,
    sync: &mut SyncCoordinator,
    mark_id: MarkID,
    new_parent_id: MarkID,
    after_id: Option<MarkID>,
    landmark_id: MarkID,
) -> EditorResult<MutationResult> {
    let edge = find_parent_edge(db, mark_id).await?;

    let new_order = compute_order_excluding(db, new_parent_id, after_id, mark_id).await?;

    terrain::update_edge_by_id(db, edge.id, new_parent_id, Some(&new_order)).await?;

    let cursor = CursorState {
        block_id: mark_id,
        offset: None,
    };
    let ops = vec![Maneuver::UpdateTerrain {
        edge_id: edge.id,
        old_src: edge.src_id,
        old_order: edge.order,
        new_src: new_parent_id,
        new_order: Some(new_order),
    }];
    history.push(ops.clone(), mark_id, cursor.clone(), cursor, false);
    let marks = touched_marks(&ops);
    sync.log_local(db, ops, marks).await?;

    let blocks = projection::project_landmark(db, landmark_id, None).await?;
    Ok(MutationResult { blocks })
}
