import type { MarkData } from "@waypoints/domain";
import { emptyRichText } from "@waypoints/domain";
import type { RenderNode } from "@waypoints/cartograph";
import { describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import {
  useBlockDragDrop,
  computeDropDepth,
  INDENT_UNIT_PX,
  type MoveBlocksFn,
} from "./useBlockDragDrop";

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
  const containerEl = ref<HTMLElement | null>(null);
  const doMove = vi.fn<MoveBlocksFn>(async () => {});
  const dd = useBlockDragDrop(blocks, containerEl, doMove);
  return { blocks, containerEl, doMove, dd };
}

describe("useBlockDragDrop", () => {
  describe("initial state", () => {
    it("starts with no dragging", () => {
      const { dd } = createDragDrop();
      expect(dd.blockIDs.value).toEqual([]);
      expect(dd.targetID.value).toBeNull();
      expect(dd.isDragging()).toBe(false);
    });

    it("dropDepth defaults to 0", () => {
      const { dd } = createDragDrop();
      expect(dd.dropDepth.value).toBe(0);
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

describe("computeDropDepth", () => {
  const containerLeft = 100;

  it("rounds cursor position to nearest depth level", () => {
    // Cursor at 112px from page left → 12px into container → 12/24 = 0.5 → rounds to 1
    expect(computeDropDepth(112, containerLeft, 1, 0)).toBe(1);

    // Cursor at 136px → 36px into container → 36/24 = 1.5 → rounds to 2
    expect(computeDropDepth(136, containerLeft, 2, 0)).toBe(2);

    // Cursor at 105px → 5px into container → 5/24 ≈ 0.2 → rounds to 0
    expect(computeDropDepth(105, containerLeft, 1, 0)).toBe(0);
  });

  it("clamps to maxDepth", () => {
    // maxDepth is 1, cursor far right
    expect(computeDropDepth(200, containerLeft, 1, 0)).toBe(1);

    // maxDepth is 3
    expect(computeDropDepth(200, containerLeft, 3, 0)).toBe(3);
  });

  it("clamps to min depth (compound scope)", () => {
    // Min depth is 1 (inside a compound), cursor at far left
    expect(computeDropDepth(100, containerLeft, 2, 1)).toBe(1);
  });

  it("returns 0 when cursor is at container left edge with no constraints", () => {
    expect(computeDropDepth(100, containerLeft, 1, 0)).toBe(0);
  });

  it("handles negative relative X (cursor left of container)", () => {
    // Cursor to the left of the container → negative relative X
    // Clamped to minDepth (0)
    expect(computeDropDepth(80, containerLeft, 1, 0)).toBe(0);
  });

  it("respects both min and max simultaneously", () => {
    // Min 1, max 2
    // Cursor at 124px → 24px → depth 1
    expect(computeDropDepth(124, containerLeft, 2, 1)).toBe(1);

    // Cursor at 148px → 48px → depth 2
    expect(computeDropDepth(148, containerLeft, 2, 1)).toBe(2);

    // Cursor at 200px → clamp to max 2
    expect(computeDropDepth(200, containerLeft, 2, 1)).toBe(2);
  });

  it("prevents indent when maxDepth equals preceding depth (non-nestable)", () => {
    // Preceding block is a heading (depth 0), not nestable → maxDepth = 0
    // Even with cursor far right, can't go deeper
    expect(computeDropDepth(200, containerLeft, 0, 0)).toBe(0);
  });
});
