import { describe, it, expect, vi, beforeEach } from "vitest";
import { ref, nextTick } from "vue";
import type { RenderNode } from "@waypoints/cartograph";
import type { MarkData } from "@waypoints/domain";
import { emptyRichText } from "@waypoints/domain";
import { useBlockFocus, type EditorHandle } from "./useBlockFocus";
import { useBlockSelection } from "./useBlockSelection";
import { useBlockKeymap } from "./useBlockKeymap";
import type { OrchestratorContext } from "./useBlockOrchestrator";

// ── Helpers ─────────────────────────────────────────────────────────────

function paragraph(id: string, parentId: string | null = null): RenderNode {
  return {
    id,
    data: { mark: "content", type: "paragraph", rich_text: emptyRichText } as MarkData,
    depth: parentId ? 1 : 0,
    fractional_index: "",
    parent_id: parentId,
    is_leaf_landmark: false,
  };
}

function mockHandle(overrides: Partial<EditorHandle> = {}): EditorHandle {
  return {
    type: "inline",
    focus: vi.fn(),
    blur: vi.fn(),
    onCoords: vi.fn(),
    onCoordsRange: vi.fn(),
    getCursorOffset: vi.fn(() => 0),
    isCursorAtStart: vi.fn(() => false),
    isCursorAtEnd: vi.fn(() => false),
    isCursorOnFirstLine: vi.fn(() => false),
    isCursorOnLastLine: vi.fn(() => false),
    getContentLength: vi.fn(() => 10),
    flush: vi.fn(async () => {}),
    canUndo: vi.fn(() => false),
    canRedo: vi.fn(() => false),
    undo: vi.fn(),
    redo: vi.fn(),
    ...overrides,
  };
}

function keyEvent(
  key: string,
  mods: Partial<Pick<KeyboardEvent, "metaKey" | "ctrlKey" | "shiftKey" | "altKey">> = {},
): KeyboardEvent {
  return new KeyboardEvent("keydown", {
    key,
    metaKey: mods.metaKey ?? false,
    ctrlKey: mods.ctrlKey ?? false,
    shiftKey: mods.shiftKey ?? false,
    altKey: mods.altKey ?? false,
    bubbles: true,
    cancelable: true,
  });
}

function createContext(blockList: RenderNode[] = []): OrchestratorContext {
  const blocks = ref(blockList);
  const focus = useBlockFocus(blocks);
  const selection = useBlockSelection(blocks, focus);

  return {
    focus,
    selection,
    blocks,
    landmarkID: ref("lm1"),
    getSiblings: vi.fn(() => []),
    getBlockIndex: vi.fn((id) => blocks.value.findIndex((b) => b.id === id)),
    getBlockAbove: vi.fn((id) => {
      const idx = blocks.value.findIndex((b) => b.id === id);
      return idx > 0 ? blocks.value[idx - 1]! : null;
    }),
    getBlockBelow: vi.fn((id) => {
      const idx = blocks.value.findIndex((b) => b.id === id);
      return idx >= 0 && idx < blocks.value.length - 1 ? blocks.value[idx + 1]! : null;
    }),
    onUpdateBlockText: vi.fn(),
    onSplitBlock: vi.fn(),
    onMergeBlocks: vi.fn(),
    onDeleteBlocks: vi.fn(),
    onUndo: vi.fn(),
    onRedo: vi.fn(),
    onCreateFirstBlock: vi.fn(),
    onCreateBlockAfter: vi.fn(),
    onIndentBlock: vi.fn(),
    onDedentBlock: vi.fn(),
    onMoveBlockUp: vi.fn(),
    onMoveBlockDown: vi.fn(),
    onMoveBlock: vi.fn(),
    onChangeBlockType: vi.fn(),
    onExitTop: undefined,
    dragDrop: {} as any,
  } as unknown as OrchestratorContext;
}

// ── Tests ───────────────────────────────────────────────────────────────

describe("useBlockKeymap", () => {
  // useKbd swaps ⌘ to ctrlKey on non-Mac; happy-dom doesn't report as Mac
  const cmdKey = () => (/Mac/.test(navigator.userAgent) ? { metaKey: true } : { ctrlKey: true });

  describe("global shortcuts", () => {
    it("⌘+Z triggers undo", () => {
      const ctx = createContext();
      const { handleKeydown } = useBlockKeymap(ctx);

      handleKeydown(keyEvent("z", cmdKey()));
      expect(ctx.onUndo).toHaveBeenCalled();
    });

    it("⌘+⇧+Z triggers redo", () => {
      const ctx = createContext();
      const { handleKeydown } = useBlockKeymap(ctx);

      handleKeydown(keyEvent("Z", { ...cmdKey(), shiftKey: true }));
      expect(ctx.onRedo).toHaveBeenCalled();
    });

    it("⌘+Y triggers redo", () => {
      const ctx = createContext();
      const { handleKeydown } = useBlockKeymap(ctx);

      handleKeydown(keyEvent("y", cmdKey()));
      expect(ctx.onRedo).toHaveBeenCalled();
    });

    it("global shortcuts fire regardless of focus/selection state", () => {
      const ctx = createContext();
      const { handleKeydown } = useBlockKeymap(ctx);

      // No focus, no selection — global should still fire
      handleKeydown(keyEvent("z", cmdKey()));
      expect(ctx.onUndo).toHaveBeenCalled();
    });
  });

  describe("selection mode shortcuts", () => {
    it("skips selection shortcuts when nothing is selected", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);

      handleKeydown(keyEvent("Backspace"));
      expect(ctx.onDeleteBlocks).not.toHaveBeenCalled();
    });

    it("Enter enters edit mode on selected block", () => {
      const ctx = createContext([paragraph("p1"), paragraph("p2")]);
      const { handleKeydown } = useBlockKeymap(ctx);

      ctx.selection.select("p1");
      handleKeydown(keyEvent("Enter"));

      expect(ctx.focus.blockID.value).toBe("p1");
    });

    it("Escape clears selection", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);

      ctx.selection.select("p1");
      expect(ctx.selection.hasSingleSelect.value).toBe(true);

      handleKeydown(keyEvent("Escape"));
      expect(ctx.selection.selectedIDs.value).toHaveLength(0);
    });

    it("Backspace deletes selected blocks", () => {
      const ctx = createContext([paragraph("p1"), paragraph("p2")]);
      const { handleKeydown } = useBlockKeymap(ctx);

      ctx.selection.select("p2");
      handleKeydown(keyEvent("Backspace"));

      expect(ctx.onDeleteBlocks).toHaveBeenCalledWith(["p2"]);
    });

    it("ArrowDown in single-select moves selection down", () => {
      const ctx = createContext([paragraph("p1"), paragraph("p2")]);
      const { handleKeydown } = useBlockKeymap(ctx);

      ctx.selection.select("p1");
      handleKeydown(keyEvent("ArrowDown"));

      expect(ctx.selection.selectedIDs.value).toEqual(["p2"]);
    });

    it("ArrowUp in single-select moves selection up", () => {
      const ctx = createContext([paragraph("p1"), paragraph("p2")]);
      const { handleKeydown } = useBlockKeymap(ctx);

      ctx.selection.select("p2");
      handleKeydown(keyEvent("ArrowUp"));

      expect(ctx.selection.selectedIDs.value).toEqual(["p1"]);
    });

    it("Shift+ArrowDown extends selection downward", () => {
      const ctx = createContext([paragraph("p1"), paragraph("p2"), paragraph("p3")]);
      const { handleKeydown } = useBlockKeymap(ctx);

      ctx.selection.select("p1");
      handleKeydown(keyEvent("ArrowDown", { shiftKey: true }));

      expect(ctx.selection.isSelected("p1")).toBe(true);
      expect(ctx.selection.isSelected("p2")).toBe(true);
    });
  });

  describe("focused mode shortcuts", () => {
    it("skips focused shortcuts when no block is focused", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);

      // No focus, no selection
      handleKeydown(keyEvent("Tab"));
      expect(ctx.onIndentBlock).not.toHaveBeenCalled();
    });

    it("Tab indents focused block", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle();

      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      handleKeydown(keyEvent("Tab"));
      expect(ctx.onIndentBlock).toHaveBeenCalledWith("p1");
    });

    it("Shift+Tab dedents focused block", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle();

      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      handleKeydown(keyEvent("Tab", { shiftKey: true }));
      expect(ctx.onDedentBlock).toHaveBeenCalledWith("p1");
    });

    it("Escape blurs and selects the block", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle();

      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      handleKeydown(keyEvent("Escape"));
      expect(ctx.focus.blockID.value).toBeNull();
      expect(ctx.selection.isSelected("p1")).toBe(true);
    });

    it("Enter at end of block creates block after", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle({ isCursorAtEnd: vi.fn(() => true) });

      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      handleKeydown(keyEvent("Enter"));
      expect(ctx.onCreateBlockAfter).toHaveBeenCalledWith("p1");
    });

    it("Enter in empty block creates block after", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle({ getContentLength: vi.fn(() => 0) });

      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      handleKeydown(keyEvent("Enter"));
      expect(ctx.onCreateBlockAfter).toHaveBeenCalledWith("p1");
    });

    it("Enter mid-content splits block", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle({
        isCursorAtEnd: vi.fn(() => false),
        getCursorOffset: vi.fn(() => 5),
      });

      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      handleKeydown(keyEvent("Enter"));
      expect(ctx.onSplitBlock).toHaveBeenCalledWith("p1", 5);
    });

    it("Backspace at start of empty block deletes it", () => {
      const ctx = createContext([paragraph("p0"), paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle({
        isCursorAtStart: vi.fn(() => true),
        getContentLength: vi.fn(() => 0),
      });

      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      handleKeydown(keyEvent("Backspace"));
      expect(ctx.onDeleteBlocks).toHaveBeenCalledWith(["p1"]);
    });

    it("Backspace at start of non-empty block merges with previous", () => {
      const ctx = createContext([paragraph("p0"), paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle({
        isCursorAtStart: vi.fn(() => true),
        getContentLength: vi.fn(() => 10),
      });

      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      handleKeydown(keyEvent("Backspace"));
      expect(ctx.onMergeBlocks).toHaveBeenCalledWith("p0", "p1");
    });

    describe("guard functions", () => {
      it("ArrowUp only fires when cursor is on first line", () => {
        const ctx = createContext([paragraph("p0"), paragraph("p1")]);
        const { handleKeydown } = useBlockKeymap(ctx);

        // Guard fails — cursor not on first line
        const handle = mockHandle({ isCursorOnFirstLine: vi.fn(() => false) });
        ctx.focus.registerEditor("p1", handle);
        ctx.focus.focusBlock("p1");

        handleKeydown(keyEvent("ArrowUp"));
        expect(ctx.focus.blockID.value).toBe("p1"); // didn't navigate away
      });

      it("ArrowDown only fires when cursor is on last line", () => {
        const ctx = createContext([paragraph("p0"), paragraph("p1")]);
        const { handleKeydown } = useBlockKeymap(ctx);

        const handle = mockHandle({ isCursorOnLastLine: vi.fn(() => true) });
        ctx.focus.registerEditor("p0", handle);
        ctx.focus.focusBlock("p0");

        handleKeydown(keyEvent("ArrowDown"));
        expect(ctx.focus.blockID.value).toBe("p1");
      });

      it("ArrowLeft only fires when cursor is at start", () => {
        const ctx = createContext([paragraph("p0"), paragraph("p1")]);
        const { handleKeydown } = useBlockKeymap(ctx);

        const handle = mockHandle({ isCursorAtStart: vi.fn(() => true) });
        ctx.focus.registerEditor("p1", handle);
        ctx.focus.focusBlock("p1");

        handleKeydown(keyEvent("ArrowLeft"));
        expect(ctx.focus.blockID.value).toBe("p0");
      });

      it("ArrowRight only fires when cursor is at end", () => {
        const ctx = createContext([paragraph("p0"), paragraph("p1")]);
        const { handleKeydown } = useBlockKeymap(ctx);

        const handle = mockHandle({ isCursorAtEnd: vi.fn(() => true) });
        ctx.focus.registerEditor("p0", handle);
        ctx.focus.focusBlock("p0");

        handleKeydown(keyEvent("ArrowRight"));
        expect(ctx.focus.blockID.value).toBe("p1");
      });
    });
  });

  describe("event handling", () => {
    it("calls preventDefault and stopPropagation on matched shortcut", () => {
      const ctx = createContext();
      const { handleKeydown } = useBlockKeymap(ctx);

      const e = keyEvent("z", cmdKey());
      const preventSpy = vi.spyOn(e, "preventDefault");
      const stopSpy = vi.spyOn(e, "stopPropagation");

      handleKeydown(e);
      expect(preventSpy).toHaveBeenCalled();
      expect(stopSpy).toHaveBeenCalled();
    });

    it("does not preventDefault on unmatched keys", () => {
      const ctx = createContext([paragraph("p1")]);
      const { handleKeydown } = useBlockKeymap(ctx);
      const handle = mockHandle();
      ctx.focus.registerEditor("p1", handle);
      ctx.focus.focusBlock("p1");

      const e = keyEvent("a");
      const preventSpy = vi.spyOn(e, "preventDefault");

      handleKeydown(e);
      expect(preventSpy).not.toHaveBeenCalled();
    });
  });
});
