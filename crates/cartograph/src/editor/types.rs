use domain::{MarkData, MarkID};
use serde::{Deserialize, Serialize};
use specta::Type;

/// A flattened block node for rendering, produced by `project_landmark`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RenderNode {
    pub id: MarkID,
    pub data: MarkData,
    pub depth: u32,
    pub fractional_index: String,
    pub parent_id: Option<MarkID>,
    pub is_leaf_landmark: bool,
}

/// Cursor position within the editor.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CursorState {
    pub block_id: MarkID,
    pub offset: Option<u32>,
}

/// Result of a mutation that re-projects the block list.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MutationResult {
    pub blocks: Vec<RenderNode>,
}

/// Result of a split operation.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SplitResult {
    pub new_mark_id: MarkID,
    pub blocks: Vec<RenderNode>,
}

/// Result of a create operation.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CreateResult {
    pub new_mark_id: MarkID,
    pub blocks: Vec<RenderNode>,
}

/// Result of an undo/redo operation.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UndoRedoResult {
    pub blocks: Vec<RenderNode>,
    pub cursor: Option<CursorState>,
}
