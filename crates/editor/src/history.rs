use std::time::Instant;

use domain::mark::MarkID;
use store::DatabaseConnection;

use crate::error::EditorResult;
use crate::ops::AtomicOp;
use crate::types::CursorState;

const MAX_HISTORY_DEPTH: usize = 200;
const COALESCE_WINDOW_MS: u128 = 500;

/// A group of atomic operations that are undone/redone together.
#[derive(Debug, Clone)]
pub struct CommandGroup {
    pub ops: Vec<AtomicOp>,
    pub last_timestamp: Instant,
    pub primary_mark_id: MarkID,
    pub cursor_before: CursorState,
    pub cursor_after: CursorState,
}

/// Per-landmark undo/redo history.
pub struct CommandHistory {
    undo_stack: Vec<CommandGroup>,
    redo_stack: Vec<CommandGroup>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Push an operation. Text edits to the same mark within the coalesce window
    /// are merged into the current group; structural ops always start a new group.
    pub fn push(
        &mut self,
        ops: Vec<AtomicOp>,
        primary_mark_id: MarkID,
        cursor_before: CursorState,
        cursor_after: CursorState,
        is_text_edit: bool,
    ) {
        let now = Instant::now();

        // Try coalescing with the last group for text edits
        if is_text_edit {
            if let Some(last) = self.undo_stack.last_mut() {
                let elapsed = now.duration_since(last.last_timestamp).as_millis();
                if last.primary_mark_id == primary_mark_id && elapsed < COALESCE_WINDOW_MS {
                    last.ops.extend(ops);
                    last.last_timestamp = now;
                    last.cursor_after = cursor_after;
                    self.redo_stack.clear();
                    return;
                }
            }
        }

        self.undo_stack.push(CommandGroup {
            ops,
            last_timestamp: now,
            primary_mark_id,
            cursor_before,
            cursor_after,
        });

        // Trim history
        if self.undo_stack.len() > MAX_HISTORY_DEPTH {
            self.undo_stack.remove(0);
        }

        self.redo_stack.clear();
    }

    /// Undo the last group: invert each op, apply in reverse, move to redo stack.
    /// Returns the cursor state to restore (cursor_before of the undone group).
    pub async fn undo(&mut self, db: &DatabaseConnection) -> EditorResult<Option<CursorState>> {
        let group = match self.undo_stack.pop() {
            Some(g) => g,
            None => return Ok(None),
        };

        let cursor = group.cursor_before.clone();

        // Apply inverted ops in reverse order
        for op in group.ops.iter().rev() {
            op.invert().apply(db).await?;
        }

        self.redo_stack.push(group);
        Ok(Some(cursor))
    }

    /// Redo the last undone group: re-apply ops, move back to undo stack.
    /// Returns the cursor state to restore (cursor_after of the redone group).
    pub async fn redo(&mut self, db: &DatabaseConnection) -> EditorResult<Option<CursorState>> {
        let group = match self.redo_stack.pop() {
            Some(g) => g,
            None => return Ok(None),
        };

        let cursor = group.cursor_after.clone();

        // Re-apply ops in forward order
        for op in &group.ops {
            op.apply(db).await?;
        }

        self.undo_stack.push(group);
        Ok(Some(cursor))
    }
}
