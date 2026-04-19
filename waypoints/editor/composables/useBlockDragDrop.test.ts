import { describe, it, expect, vi, beforeEach } from "vitest";
import { ref } from "vue";
import type { RenderNode } from "@waypoints/editor";
import type { MarkData } from "@waypoints/domain";
import { emptyRichText } from "@waypoints/domain";
import {
  useBlockDragDrop,
  computeDropDepth,
  type MoveBlocksFn,
} from "./useBlockDragDrop";

// ── Helpers ─────────────────────────────────────────────────────────────

function paragraph(
  id: string,
  parentId: string | null = null,
  depth = parentId ? 1 : 0,
): RenderNode {
  return {
    id,
    data: { mark: "content", type: "paragraph", rich_text: emptyRichText } as MarkData,
    depth,
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

function createDragDrop(blockList: RenderNode[] = []) {
  const blocks = ref(blockList);
  const doMove = vi.fn<MoveBlocksFn>(async () => {});
  const dd = useBlockDragDrop(blocks, doMove);
  return { blocks, doMove, dd };
}

// ── Tests ───────────────────────────────────────────────────────────────

describe("useBlockDragDrop", () => {
  describe("initial state", () => {
    it("starts with no dragging", () => {
      const { dd } = createDragDrop();
      expect(dd.blockIDs.value).toEqual([]);
      expect(dd.targetID.value).toBeNull();
      expect(dd.isDragging()).toBe(false);
    });
  });

  describe("isDragging", () => {
    it("returns false when not dragging", () => {
      const { dd } = createDragDrop();
      expect(dd.isDragging()).toBe(false);
      expect(dd.isDragging("p1")).toBe(false);
    });
  });

  describe("isTarget", () => {
    it("returns false when no target", () => {
      const { dd } = createDragDrop();
      expect(dd.isTarget("p1")).toBe(false);
    });
  });

  describe("position helpers", () => {
    it("getBlockPosition defaults to 'after'", () => {
      const { dd } = createDragDrop();
      expect(dd.getBlockPosition()).toBe("after");
    });

    it("isAfter/isBefore/isInside reflect position state", () => {
      const { dd } = createDragDrop();
      // Default position is "after"
      expect(dd.isAfter()).toBe(true);
      expect(dd.isBefore()).toBe(false);
      expect(dd.isInside()).toBe(false);
    });
  });

  describe("showDropTarget", () => {
    it("returns false when not dragging", () => {
      const { dd } = createDragDrop();
      expect(dd.showDropTarget("p1", "before")).toBe(false);
      expect(dd.showDropTarget("p1", "after")).toBe(false);
    });
  });
});
