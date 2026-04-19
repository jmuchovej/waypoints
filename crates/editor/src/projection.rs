use std::collections::{HashMap, HashSet};

use domain::mark::{Content, Mark, MarkData, MarkID, RichText, TextMarkKind};
use store::DatabaseConnection;
use store::query::{mark, terrain};

use crate::error::{EditorError, EditorResult};
use crate::types::RenderNode;

const MAX_DEPTH: u32 = 20;

/// Project a landmark into a flat list of `RenderNode`s with depth tracking.
pub async fn project_landmark(
    db: &DatabaseConnection,
    landmark_id: MarkID,
    max_depth: Option<u32>,
) -> EditorResult<Vec<RenderNode>> {
    let m = mark::get(db, landmark_id)
        .await?
        .ok_or(EditorError::NotFound(landmark_id.inner()))?;

    if !m.is_landmark() {
        return Err(EditorError::NotLandmark(landmark_id.inner()));
    }

    let cap = max_depth.unwrap_or(MAX_DEPTH);
    let mut nodes = Vec::new();
    walk_children(db, landmark_id, 0, cap, &mut nodes).await?;
    hydrate_mentions(db, &mut nodes).await?;
    Ok(nodes)
}

/// Recursively walk child terrain edges and collect `RenderNode`s.
async fn walk_children(
    db: &DatabaseConnection,
    parent_id: MarkID,
    depth: u32,
    max_depth: u32,
    out: &mut Vec<RenderNode>,
) -> EditorResult<()> {
    if depth >= max_depth {
        return Ok(());
    }

    // Query child edges ordered by fractional index
    let edges = terrain::get_child_edges(db, parent_id).await?;

    if edges.is_empty() {
        return Ok(());
    }

    // Batch-fetch child marks
    let dst_ids: Vec<uuid::Uuid> = edges.iter().map(|e| e.dst_id.inner()).collect();
    let children = mark::get_many(db, &dst_ids).await?;

    // Index by ID for ordering
    let child_map: HashMap<MarkID, Mark> = children.into_iter().map(|m| (m.id, m)).collect();

    // Iterate in terrain order
    for edge in &edges {
        let Some(child) = child_map.get(&edge.dst_id) else {
            continue;
        };

        let is_leaf_landmark = child.is_landmark();
        let fractional_index = edge.order.clone().unwrap_or_default();

        out.push(RenderNode {
            id: child.id,
            data: child.data.clone(),
            depth,
            fractional_index,
            parent_id: Some(parent_id),
            is_leaf_landmark,
        });

        // Recurse into content children, but not nested landmarks
        if !is_leaf_landmark {
            Box::pin(walk_children(db, child.id, depth + 1, max_depth, out)).await?;
        }
    }

    Ok(())
}

/// Extract the UUID from a mention's `data` field (a JSON string containing a UUID).
fn mention_id(data: &Option<domain::Json>) -> Option<uuid::Uuid> {
    let s = data.as_ref()?.as_str()?;
    uuid::Uuid::parse_str(s).ok()
}

/// Collect all RichText references (immutable) from a RenderNode's data.
fn collect_rich_texts(data: &MarkData) -> Vec<&RichText> {
    match data {
        MarkData::Content(c) => {
            if let Some(rt) = c.rich_text() {
                vec![rt]
            } else if let Content::Table { cells, .. } = c {
                cells.iter().flatten().collect()
            } else {
                vec![]
            }
        }
        MarkData::Landmark(l) => vec![&l.title],
    }
}

/// Collect all RichText references (mutable) from a RenderNode's data.
fn collect_rich_texts_mut(data: &mut MarkData) -> Vec<&mut RichText> {
    match data {
        MarkData::Content(c) => {
            if let Content::Table { cells, .. } = c {
                cells.iter_mut().flatten().collect()
            } else if let Some(rt) = c.rich_text_mut() {
                vec![rt]
            } else {
                vec![]
            }
        }
        MarkData::Landmark(l) => vec![&mut l.title],
    }
}

/// Hydrate mention marks in-place: replace bare UUID data with `{ id, mark }`
/// and set segment text to the referenced landmark's title.
async fn hydrate_mentions(db: &DatabaseConnection, nodes: &mut [RenderNode]) -> EditorResult<()> {
    // Pass 1: collect unique mention IDs across all nodes.
    let mut ids = HashSet::new();
    for node in nodes.iter() {
        for rt in collect_rich_texts(&node.data) {
            for seg in rt.segments() {
                for m in &seg.marks {
                    if m.kind == TextMarkKind::Mention {
                        if let Some(uuid) = mention_id(&m.data) {
                            ids.insert(uuid);
                        }
                    }
                }
            }
        }
    }

    if ids.is_empty() {
        return Ok(());
    }

    // Batch-fetch referenced marks.
    let id_vec: Vec<uuid::Uuid> = ids.into_iter().collect();
    let marks = mark::get_many(db, &id_vec).await?;
    let mark_map: HashMap<uuid::Uuid, Mark> =
        marks.into_iter().map(|m| (m.id.inner(), m)).collect();

    // Pass 2: hydrate in-place.
    for node in nodes.iter_mut() {
        for rt in collect_rich_texts_mut(&mut node.data) {
            for seg in rt.segments_mut() {
                let has_mention = seg.marks.iter().any(|m| m.kind == TextMarkKind::Mention);
                if !has_mention {
                    continue;
                }

                for m in &mut seg.marks {
                    if m.kind != TextMarkKind::Mention {
                        continue;
                    }
                    let Some(uuid) = mention_id(&m.data) else {
                        continue;
                    };

                    if let Some(referenced) = mark_map.get(&uuid) {
                        // Resolve display text: landmark title or "Untitled".
                        let title = match &referenced.data {
                            MarkData::Landmark(l) => {
                                let t = l.title.plain_text();
                                if t.is_empty() { "Untitled" } else { t }
                            }
                            _ => "Untitled",
                        };
                        seg.text = title.to_string();

                        // Hydrate data: { id, mark }.
                        let mark_json =
                            serde_json::to_value(&referenced).unwrap_or(serde_json::Value::Null);
                        m.data = Some(serde_json::json!({
                            "id": uuid.to_string(),
                            "mark": mark_json,
                        }));
                    } else {
                        // Referenced mark not found — preserve ID, show fallback text.
                        seg.text = "Untitled".to_string();
                        m.data = Some(serde_json::json!({
                            "id": uuid.to_string(),
                        }));
                    }
                }
            }
        }
    }

    Ok(())
}
