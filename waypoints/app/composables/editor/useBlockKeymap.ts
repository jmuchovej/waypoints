import { createLogger } from "~/utils/logger";
import type { EditorHandle } from "./useBlockFocus";
import type { OrchestratorContext } from "./useBlockOrchestrator";
import { useEditorKeybind } from "./useKeybinds";

const log = createLogger("keymap");

export type BlockShortcutMode = "global" | "focused" | "selection";

export interface BlockShortcut {
  /** Key combo: "⌘ + Z", "⇧ + ArrowUp", "Enter" */
  key: string;
  /** When shortcut is active. Default: "focused" */
  mode?: BlockShortcutMode;
  /** Cursor-state predicate (focused mode only). Must return true to fire. */
  guard?: (handle: EditorHandle) => boolean;
  /** Handler. Called after preventDefault + stopPropagation. */
  handler: (ctx: OrchestratorContext, e: KeyboardEvent) => void;
}

function defineShortcuts(ctx: OrchestratorContext): BlockShortcut[] {
  return [
    // ── Global ────────────────────────────────────────────────────────
    {
      key: "⌘ + Z",
      mode: "global",
      handler: () => ctx.onUndo(),
    },
    {
      key: "⌘ + ⇧ + Z",
      mode: "global",
      handler: () => ctx.onRedo(),
    },
    {
      key: "⌘ + Y",
      mode: "global",
      handler: () => ctx.onRedo(),
    },

    // ── Selection (single or multi-block) ─────────────────────────────
    {
      key: "Enter",
      mode: "selection",
      handler: () => {
        const ids = ctx.selection.selectedIDs.value;
        if (ids.length === 0) return;
        ctx.selection.clear();
        ctx.focus.focusBlock(ids[0]!, "start");
      },
    },
    {
      key: "Backspace",
      mode: "selection",
      handler: () => {
        const ids = ctx.selection.selectedIDs.value;
        if (ids.length === 0) return;
        const firstIdx = ctx.getBlockIndex(ids[0]!);
        const prev = firstIdx > 0 ? ctx.blocks.value[firstIdx - 1] : null;
        ctx.onDeleteBlocks(ids);
        ctx.selection.clear();
        if (prev) ctx.focus.focusBlock(prev.id, "end");
      },
    },
    {
      key: "Delete",
      mode: "selection",
      handler: () => {
        const ids = ctx.selection.selectedIDs.value;
        if (ids.length === 0) return;
        const firstIdx = ctx.getBlockIndex(ids[0]!);
        const prev = firstIdx > 0 ? ctx.blocks.value[firstIdx - 1] : null;
        ctx.onDeleteBlocks(ids);
        ctx.selection.clear();
        if (prev) ctx.focus.focusBlock(prev.id, "end");
      },
    },
    {
      key: "Escape",
      mode: "selection",
      handler: () => ctx.selection.clear(),
    },
    {
      key: "⇧ + ArrowUp",
      mode: "selection",
      handler: () => ctx.selection.extend("up"),
    },
    {
      key: "⇧ + ArrowDown",
      mode: "selection",
      handler: () => ctx.selection.extend("down"),
    },
    {
      key: "ArrowUp",
      mode: "selection",
      handler: () => {
        if (ctx.selection.hasMultiSelect.value) {
          // Multi → collapse to first block as single-select
          const ids = ctx.selection.selectedIDs.value;
          if (ids.length > 0) ctx.selection.select(ids[0]!);
        } else {
          // Single → move cursor up
          ctx.selection.move("up");
        }
      },
    },
    {
      key: "ArrowDown",
      mode: "selection",
      handler: () => {
        if (ctx.selection.hasMultiSelect.value) {
          // Multi → collapse to last block as single-select
          const ids = ctx.selection.selectedIDs.value;
          if (ids.length > 0) ctx.selection.select(ids[ids.length - 1]!);
        } else {
          // Single → move cursor down
          ctx.selection.move("down");
        }
      },
    },

    // ── Focused (single block, editing) ───────────────────────────────
    {
      key: "⇧ + ArrowUp",
      mode: "focused",
      handler: () => ctx.selection.extend("up"),
    },
    {
      key: "⇧ + ArrowDown",
      mode: "focused",
      handler: () => ctx.selection.extend("down"),
    },
    {
      key: "⌘ + ⇧ + ArrowUp",
      mode: "focused",
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (blockId) ctx.onMoveBlockUp(blockId);
      },
    },
    {
      key: "⌘ + ⇧ + ArrowDown",
      mode: "focused",
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (blockId) ctx.onMoveBlockDown(blockId);
      },
    },
    {
      key: "Enter",
      mode: "focused",
      guard: (h) => h.isCursorAtEnd() || h.getContentLength() === 0,
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (blockId) ctx.onCreateBlockAfter(blockId);
      },
    },
    {
      key: "Enter",
      mode: "focused",
      guard: (h) => !h.isCursorAtEnd(),
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (!blockId) return;
        const handle = ctx.focus.getHandle(blockId);
        if (!handle) return;
        ctx.onSplitBlock(blockId, handle.getCursorOffset());
      },
    },
    {
      key: "Backspace",
      mode: "focused",
      guard: (h) => h.isCursorAtStart() && h.getContentLength() === 0,
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (!blockId) return;
        const prev = ctx.getBlockAbove(blockId);
        ctx.onDeleteBlocks([blockId]);
        if (prev) {
          ctx.focus.focusBlock(prev.id, "end");
        } else if (ctx.onExitTop) {
          ctx.onExitTop();
        }
      },
    },
    {
      key: "Backspace",
      mode: "focused",
      guard: (h) => h.isCursorAtStart() && h.getContentLength() > 0,
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (!blockId) return;
        const prev = ctx.getBlockAbove(blockId);
        if (prev) {
          ctx.onMergeBlocks(prev.id, blockId);
        } else if (ctx.onExitTop) {
          ctx.focus.blurAll();
          ctx.onExitTop();
        }
      },
    },
    {
      key: "Tab",
      mode: "focused",
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (blockId) ctx.onIndentBlock(blockId);
      },
    },
    {
      key: "⇧ + Tab",
      mode: "focused",
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (blockId) ctx.onDedentBlock(blockId);
      },
    },
    {
      key: "ArrowLeft",
      mode: "focused",
      guard: (h) => h.isCursorAtStart(),
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (!blockId) return;
        const prev = ctx.getBlockAbove(blockId);
        if (prev) {
          ctx.focus.focusBlock(prev.id, "end");
        } else if (ctx.onExitTop) {
          ctx.focus.blurAll();
          ctx.onExitTop();
        }
      },
    },
    {
      key: "ArrowRight",
      mode: "focused",
      guard: (h) => h.isCursorAtEnd(),
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (!blockId) return;
        const next = ctx.getBlockBelow(blockId);
        if (next) ctx.focus.focusBlock(next.id, "start");
      },
    },
    {
      key: "ArrowUp",
      mode: "focused",
      guard: (h) => h.isCursorOnFirstLine(),
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (!blockId) return;
        const prev = ctx.getBlockAbove(blockId);
        if (prev) {
          ctx.focus.focusBlock(prev.id, "end");
        } else if (ctx.onExitTop) {
          ctx.focus.blurAll();
          ctx.onExitTop();
        }
      },
    },
    {
      key: "ArrowDown",
      mode: "focused",
      guard: (h) => h.isCursorOnLastLine(),
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        if (!blockId) return;
        const next = ctx.getBlockBelow(blockId);
        if (next) ctx.focus.focusBlock(next.id, "start");
      },
    },
    {
      key: "Escape",
      mode: "focused",
      handler: () => {
        const blockId = ctx.focus.blockID.value;
        ctx.focus.blurAll();
        if (blockId) ctx.selection.select(blockId);
      },
    },
  ];
}

export function useBlockKeymap(ctx: OrchestratorContext) {
  const { matchKeyCombo, parseKeyCombo } = useEditorKeybind();

  const rawShortcuts = defineShortcuts(ctx);

  // Pre-parse all combos once
  const shortcuts = rawShortcuts.map((s) => ({
    ...s,
    parsed: parseKeyCombo(s.key),
  }));

  function handleKeydown(e: KeyboardEvent) {
    if (!e.key) return;

    const hasSelection = ctx.selection.hasSingleSelect.value;
    const blockId = ctx.focus.blockID.value;
    const handle = blockId ? ctx.focus.getHandle(blockId) : undefined;

    log.debug("keydown", {
      key: e.key,
      shift: e.shiftKey,
      meta: e.metaKey,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      hasSelection,
      blockId,
      hasHandle: !!handle,
    });

    for (const shortcut of shortcuts) {
      if (!matchKeyCombo(e, shortcut.parsed)) continue;

      const mode = shortcut.mode ?? "focused";

      if (mode === "selection" && !hasSelection) {
        log.debug("skip (selection mode, no selection)", shortcut.key);
        continue;
      }
      if (mode === "focused" && (hasSelection || !blockId || !handle)) {
        log.debug("skip (focused mode, state mismatch)", shortcut.key, {
          hasSelection,
          blockId,
          hasHandle: !!handle,
        });
        continue;
      }

      if (mode === "focused" && shortcut.guard && !shortcut.guard(handle!)) {
        log.debug("skip (guard failed)", shortcut.key);
        continue;
      }

      log.debug("matched →", shortcut.key, `[${mode}]`);
      e.preventDefault();
      e.stopPropagation();
      shortcut.handler(ctx, e);
      return;
    }
    log.debug("no match, propagating to TipTap");
  }

  return { shortcuts: rawShortcuts, handleKeydown };
}
