use domain::mark::MarkID;
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
    let child_map: std::collections::HashMap<MarkID, domain::mark::Mark> =
        children.into_iter().map(|m| (m.id, m)).collect();

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
