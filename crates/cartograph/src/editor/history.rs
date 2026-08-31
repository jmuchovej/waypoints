use crate::{CursorState, EditorResult, Maneuver};
use domain::MarkID;
use std::time::Instant;
use store::Store;

const MAX_HISTORY_DEPTH: usize = 200;
const COALESCE_WINDOW_MS: u128 = 500;

/// A group of atomic operations that are undone/redone together.
#[derive(Debug, Clone)]
pub struct CommandGroup {
    pub ops: Vec<Maneuver>,
    pub last_timestamp: Instant,
    pub primary_mark_id: MarkID,
    pub cursor_before: CursorState,
    pub cursor_after: CursorState,
}

impl CommandGroup {
    /// A pure text group — one whose ops are all `UpdateRichText`. Only such
    /// groups coalesce, so a burst of typing undoes as one step while a
    /// structural group never absorbs a later text edit, even on the same
    /// mark. Derived from the ops themselves rather than tracked as a flag,
    /// so a group cannot lie about its kind.
    fn is_text_edit(&self) -> bool {
        is_text_edit(&self.ops)
    }
}

fn is_text_edit(ops: &[Maneuver]) -> bool {
    !ops.is_empty()
        && ops
            .iter()
            .all(|op| matches!(op, Maneuver::UpdateRichText { .. }))
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

    /// Push an operation group. Pure text groups (all ops `UpdateRichText`)
    /// targeting the same mark within the coalesce window merge into the
    /// current group; any other group starts a new undo step.
    pub fn push(
        &mut self,
        ops: Vec<Maneuver>,
        primary_mark_id: MarkID,
        cursor_before: CursorState,
        cursor_after: CursorState,
    ) {
        let now = Instant::now();

        // Coalesce text into text: both the incoming ops and the last group
        // must be pure text edits — a structural group on the same mark must
        // stay a separate undo step.
        if is_text_edit(&ops)
            && let Some(last) = self.undo_stack.last_mut()
        {
            let elapsed = now.duration_since(last.last_timestamp).as_millis();
            if last.is_text_edit()
                && last.primary_mark_id == primary_mark_id
                && elapsed < COALESCE_WINDOW_MS
            {
                last.ops.extend(ops);
                last.last_timestamp = now;
                last.cursor_after = cursor_after;
                self.redo_stack.clear();
                return;
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
    /// Returns the cursor state to restore (cursor_before of the undone group)
    /// and the inverted ops that were applied — the caller logs those to sync,
    /// since an undo mutates the database like any other command.
    pub async fn undo(&mut self, db: &Store) -> EditorResult<Option<(CursorState, Vec<Maneuver>)>> {
        let group = match self.undo_stack.pop() {
            Some(g) => g,
            None => return Ok(None),
        };

        let cursor = group.cursor_before.clone();

        // Apply inverted ops in reverse order
        let mut applied = Vec::with_capacity(group.ops.len());
        for op in group.ops.iter().rev() {
            let inverse = op.invert();
            inverse.apply(db).await?;
            applied.push(inverse);
        }

        self.redo_stack.push(group);
        Ok(Some((cursor, applied)))
    }

    /// Redo the last undone group: re-apply ops, move back to undo stack.
    /// Returns the cursor state to restore (cursor_after of the redone group)
    /// and the re-applied ops for sync logging.
    pub async fn redo(&mut self, db: &Store) -> EditorResult<Option<(CursorState, Vec<Maneuver>)>> {
        let group = match self.redo_stack.pop() {
            Some(g) => g,
            None => return Ok(None),
        };

        let cursor = group.cursor_after.clone();

        // Re-apply ops in forward order
        for op in &group.ops {
            op.apply(db).await?;
        }

        let applied = group.ops.clone();
        self.undo_stack.push(group);
        Ok(Some((cursor, applied)))
    }
}
