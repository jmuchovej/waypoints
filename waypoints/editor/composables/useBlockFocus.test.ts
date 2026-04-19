import { describe, it, expect, vi, beforeEach } from "vitest";
import { ref, nextTick } from "vue";
import type { RenderNode } from "@waypoints/editor";
import type { MarkData, RichText } from "@waypoints/domain";
import { useBlockFocus, type EditorHandle } from "./useBlockFocus";

// ── Helpers ─────────────────────────────────────────────────────────────

const emptyRt: RichText = { plain_text: "", segments: [] };

function paragraph(id: string, parentId: string | null = null): RenderNode {
  return {
    id,
    data: { mark: "content", type: "paragraph", rich_text: emptyRt } as MarkData,
    depth: parentId ? 1 : 0,
    fractional_index: "",
    parent_id: parentId,
    is_leaf_landmark: false,
  };
}

function blockquote(id: string): RenderNode {
  return {
    id,
    data: { mark: "content", type: "blockquote" } as MarkData,
    depth: 0,
    fractional_index: "",
    parent_id: null,
    is_leaf_landmark: false,
  };
}

function mockHandle(): EditorHandle {
  return {
    type: "inline",
    focus: vi.fn(),
    blur: vi.fn(),
    onCoords: vi.fn(),
    onCoordsRange: vi.fn(),
    getCursorOffset: vi.fn(() => 0),
    isCursorAtStart: vi.fn(() => true),
    isCursorAtEnd: vi.fn(() => false),
    isCursorOnFirstLine: vi.fn(() => true),
    isCursorOnLastLine: vi.fn(() => false),
    getContentLength: vi.fn(() => 0),
    flush: vi.fn(async () => {}),
  };
}

// ── Tests ───────────────────────────────────────────────────────────────

describe("useBlockFocus", () => {
  describe("focusBlock", () => {
    it("sets blockID", () => {
      const focus = useBlockFocus();
      focus.focusBlock("b1");
      expect(focus.blockID.value).toBe("b1");
    });

    it("blurs previous block when focusing a new one", async () => {
      const focus = useBlockFocus();
      const h1 = mockHandle();
      focus.registerEditor("b1", h1);
      focus.focusBlock("b1");
      await nextTick();

      focus.focusBlock("b2");
      expect(h1.blur).toHaveBeenCalled();
      expect(focus.blockID.value).toBe("b2");
    });

    it("calls handle.focus via nextTick when handle exists", async () => {
      const focus = useBlockFocus();
      const handle = mockHandle();
      focus.registerEditor("b1", handle);

      focus.focusBlock("b1", "end");
      expect(handle.focus).not.toHaveBeenCalled(); // not yet — queued in nextTick
      await nextTick();
      expect(handle.focus).toHaveBeenCalledWith("end");
    });

    it("defaults position to 'start'", async () => {
      const focus = useBlockFocus();
      const handle = mockHandle();
      focus.registerEditor("b1", handle);

      focus.focusBlock("b1");
      await nextTick();
      expect(handle.focus).toHaveBeenCalledWith("start");
    });
  });

  describe("pending focus intent", () => {
    it("queues intent when no handle registered yet", async () => {
      const focus = useBlockFocus();
      focus.focusBlock("b1", "end");
      expect(focus.blockID.value).toBe("b1");

      // Now register the handle — should execute the pending intent
      const handle = mockHandle();
      focus.registerEditor("b1", handle);
      await nextTick();
      expect(handle.focus).toHaveBeenCalledWith("end");
    });

    it("executes coords intent on register", async () => {
      const focus = useBlockFocus();
      const event = { clientX: 100, clientY: 200 } as MouseEvent;
      focus.focusBlockAt("b1", event);

      const handle = mockHandle();
      focus.registerEditor("b1", handle);
      await nextTick();
      expect(handle.onCoords).toHaveBeenCalledWith(100, 200);
    });

    it("does not execute intent for a different block", async () => {
      const focus = useBlockFocus();
      focus.focusBlock("b1", "start");

      const handle = mockHandle();
      focus.registerEditor("b2", handle); // registering b2, not b1
      await nextTick();
      expect(handle.focus).not.toHaveBeenCalled();
    });
  });

  describe("focusBlockDeferred", () => {
    it("always creates a pending intent even if handle exists", async () => {
      const focus = useBlockFocus();
      const handle = mockHandle();
      focus.registerEditor("b1", handle);

      focus.focusBlockDeferred("b1", "end");
      expect(focus.blockID.value).toBe("b1");
      // Intent is stored but not executed yet since the handle was already
      // registered before focusBlockDeferred was called — the intent waits
      // for a new registerEditor call (after destroy/recreate cycle)
      await nextTick();
      // handle.focus should NOT have been called — no new registerEditor
      expect(handle.focus).not.toHaveBeenCalled();

      // Simulate editor destroy + recreate (new handle registration)
      const newHandle = mockHandle();
      focus.registerEditor("b1", newHandle);
      await nextTick();
      expect(newHandle.focus).toHaveBeenCalledWith("end");
    });
  });

  describe("focusBlockAt", () => {
    it("calls onCoords when handle exists", async () => {
      const focus = useBlockFocus();
      const handle = mockHandle();
      focus.registerEditor("b1", handle);

      const event = { clientX: 50, clientY: 75 } as MouseEvent;
      focus.focusBlockAt("b1", event);
      await nextTick();
      expect(handle.onCoords).toHaveBeenCalledWith(50, 75);
    });

    it("queues coords intent when handle not registered", async () => {
      const focus = useBlockFocus();
      const event = { clientX: 50, clientY: 75 } as MouseEvent;
      focus.focusBlockAt("b1", event);

      const handle = mockHandle();
      focus.registerEditor("b1", handle);
      await nextTick();
      expect(handle.onCoords).toHaveBeenCalledWith(50, 75);
    });
  });

  describe("blurAll", () => {
    it("blurs current handle and clears blockID", () => {
      const focus = useBlockFocus();
      const handle = mockHandle();
      focus.registerEditor("b1", handle);
      focus.focusBlock("b1");

      focus.blurAll();
      expect(handle.blur).toHaveBeenCalled();
      expect(focus.blockID.value).toBeNull();
    });

    it("is a no-op when nothing is focused", () => {
      const focus = useBlockFocus();
      focus.blurAll(); // should not throw
      expect(focus.blockID.value).toBeNull();
    });
  });

  describe("unregisterEditor", () => {
    it("removes handle and clears blockID if it was focused", () => {
      const focus = useBlockFocus();
      const handle = mockHandle();
      focus.registerEditor("b1", handle);
      focus.focusBlock("b1");

      focus.unregisterEditor("b1");
      expect(focus.blockID.value).toBeNull();
      expect(focus.getHandle("b1")).toBeUndefined();
    });

    it("does not clear blockID if a different block is focused", () => {
      const focus = useBlockFocus();
      focus.registerEditor("b1", mockHandle());
      focus.registerEditor("b2", mockHandle());
      focus.focusBlock("b2");

      focus.unregisterEditor("b1");
      expect(focus.blockID.value).toBe("b2");
    });
  });

  describe("isFocused", () => {
    it("returns true for focused block", () => {
      const focus = useBlockFocus();
      focus.focusBlock("b1");
      expect(focus.isFocused("b1")).toBe(true);
      expect(focus.isFocused("b2")).toBe(false);
    });
  });

  describe("groupID", () => {
    it("returns null when no blocks provided", () => {
      const focus = useBlockFocus();
      focus.focusBlock("b1");
      expect(focus.groupID.value).toBeNull();
    });

    it("returns null for top-level block", () => {
      const blocks = ref([paragraph("b1")]);
      const focus = useBlockFocus(blocks);
      focus.focusBlock("b1");
      expect(focus.groupID.value).toBeNull();
    });

    it("returns compound parent ID for nested block", () => {
      const blocks = ref([blockquote("bq1"), paragraph("p1", "bq1")]);
      const focus = useBlockFocus(blocks);
      focus.focusBlock("p1");
      expect(focus.groupID.value).toBe("bq1");
    });

    it("returns null when parent is not compound", () => {
      // parent_id points to a paragraph (not compound)
      const blocks = ref([paragraph("p-parent"), paragraph("p-child", "p-parent")]);
      const focus = useBlockFocus(blocks);
      focus.focusBlock("p-child");
      expect(focus.groupID.value).toBeNull();
    });
  });

  describe("focusBlockWithRange", () => {
    it("sets blockID and stores range intent", () => {
      const focus = useBlockFocus();
      focus.focusBlockWithRange("b1", 10, 20, 100, 20);
      expect(focus.blockID.value).toBe("b1");
    });

    it("executes range intent on registerEditor", async () => {
      const focus = useBlockFocus();
      focus.focusBlockWithRange("b1", 10, 20, 100, 20);

      const handle = mockHandle();
      focus.registerEditor("b1", handle);
      await nextTick();

      expect(handle.onCoordsRange).toHaveBeenCalledWith(10, 20, 100, 20);
      expect(handle.onCoords).not.toHaveBeenCalled();
      expect(handle.focus).not.toHaveBeenCalled();
    });

    it("blurs previous block", () => {
      const focus = useBlockFocus();
      const prevHandle = mockHandle();
      focus.registerEditor("b1", prevHandle);
      focus.focusBlock("b1");

      focus.focusBlockWithRange("b2", 10, 20, 100, 20);
      expect(prevHandle.blur).toHaveBeenCalled();
    });
  });
});
