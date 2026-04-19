import { describe, it, expect, vi, beforeEach } from "vitest";
import { ref, nextTick } from "vue";
import type { RenderNode } from "@waypoints/cartograph";
import type { MarkData } from "@waypoints/domain";
import { emptyRichText } from "@waypoints/domain";
import { useBlockFocus, type EditorHandle } from "./useBlockFocus";
import { useBlockSelection } from "./useBlockSelection";

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

// Standard flat block list: p1, p2, p3, p4
function flatBlocks() {
  return ref([paragraph("p1"), paragraph("p2"), paragraph("p3"), paragraph("p4")]);
}

// Compound block list: p1, bq(c1, c2, c3), p2
function compoundBlocks() {
  return ref([
    paragraph("p1"),
    blockquote("bq"),
    paragraph("c1", "bq"),
    paragraph("c2", "bq"),
    paragraph("c3", "bq"),
    paragraph("p2"),
  ]);
}

// ── Tests ───────────────────────────────────────────────────────────────

describe("useBlockSelection", () => {
  describe("select / clear / toggle", () => {
    it("selects a single block", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p2");
      expect(sel.isSelected("p2")).toBe(true);
      expect(sel.hasSingleSelect.value).toBe(true);
      expect(sel.hasMultiSelect.value).toBe(false);
    });

    it("clears selection", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p2");
      sel.clear();
      expect(sel.selectedIDs.value).toHaveLength(0);
    });

    it("toggles a block in and out of selection", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.toggle("p1");
      expect(sel.isSelected("p1")).toBe(true);

      sel.toggle("p2");
      expect(sel.isSelected("p1")).toBe(true);
      expect(sel.isSelected("p2")).toBe(true);
      expect(sel.hasMultiSelect.value).toBe(true);

      sel.toggle("p1");
      expect(sel.isSelected("p1")).toBe(false);
      expect(sel.isSelected("p2")).toBe(true);
    });
  });

  describe("range", () => {
    it("selects contiguous range inclusive", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p1"); // sets anchor
      sel.range("p1", "p3");

      expect(sel.selectedIDs.value).toHaveLength(3);
      expect(sel.isSelected("p1")).toBe(true);
      expect(sel.isSelected("p2")).toBe(true);
      expect(sel.isSelected("p3")).toBe(true);
      expect(sel.isSelected("p4")).toBe(false);
    });

    it("works with reversed order (dst before src)", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p3");
      sel.range("p3", "p1");

      expect(sel.selectedIDs.value).toHaveLength(3);
      expect(sel.isSelected("p1")).toBe(true);
      expect(sel.isSelected("p3")).toBe(true);
    });

    it("no-ops for invalid IDs", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.range("missing1", "missing2");
      expect(sel.selectedIDs.value).toHaveLength(0);
    });
  });

  describe("shiftClick", () => {
    it("ranges from anchor to target", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p1"); // sets anchor
      sel.shiftClick("p3");

      expect(sel.selectedIDs.value).toHaveLength(3);
    });

    it("uses focused block as anchor when no anchor set", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      focus.focusBlock("p2");
      sel.shiftClick("p4");

      expect(sel.selectedIDs.value).toHaveLength(3);
      expect(sel.isSelected("p2")).toBe(true);
      expect(sel.isSelected("p3")).toBe(true);
      expect(sel.isSelected("p4")).toBe(true);
    });

    it("selects only the target when no anchor and no focus", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.shiftClick("p3");
      expect(sel.selectedIDs.value).toEqual(["p3"]);
    });
  });

  describe("move", () => {
    it("moves selection down", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p1");
      sel.move("down");
      expect(sel.selectedIDs.value).toEqual(["p2"]);
    });

    it("moves selection up", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p3");
      sel.move("up");
      expect(sel.selectedIDs.value).toEqual(["p2"]);
    });

    it("does not move past top boundary", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p1");
      sel.move("up");
      // Still on p1 (no compound parent to pop to)
      expect(sel.selectedIDs.value).toEqual(["p1"]);
    });

    it("does not move past bottom boundary", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p4");
      sel.move("down");
      expect(sel.selectedIDs.value).toEqual(["p4"]);
    });
  });

  describe("extend", () => {
    it("extends selection downward from focused block", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      focus.focusBlock("p2");
      sel.extend("down");

      expect(sel.isSelected("p2")).toBe(true);
      expect(sel.isSelected("p3")).toBe(true);
      expect(sel.selectedIDs.value).toHaveLength(2);
    });

    it("extends selection upward", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      focus.focusBlock("p3");
      sel.extend("up");

      expect(sel.isSelected("p2")).toBe(true);
      expect(sel.isSelected("p3")).toBe(true);
    });

    it("shrinks selection when extending opposite direction from anchor", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      // Start at p2, extend down twice (p2, p3, p4)
      sel.select("p2"); // anchor = p2
      sel.extend("down"); // p2, p3
      sel.extend("down"); // p2, p3, p4

      expect(sel.selectedIDs.value).toHaveLength(3);

      // Now extend up — should shrink from bottom
      sel.extend("up");
      expect(sel.selectedIDs.value).toHaveLength(2);
      expect(sel.isSelected("p4")).toBe(false);
    });

    it("is a no-op when nothing focused and nothing selected", () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.extend("down");
      expect(sel.selectedIDs.value).toHaveLength(0);
    });
  });

  describe("compound scope awareness", () => {
    it("range within compound stays scoped to compound children", () => {
      const blocks = compoundBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("c1"); // anchor inside compound
      sel.range("c1", "c3");

      expect(sel.selectedIDs.value).toHaveLength(3);
      expect(sel.isSelected("c1")).toBe(true);
      expect(sel.isSelected("c2")).toBe(true);
      expect(sel.isSelected("c3")).toBe(true);
      // Top-level blocks should not be selected
      expect(sel.isSelected("p1")).toBe(false);
      expect(sel.isSelected("p2")).toBe(false);
    });

    it("move within compound stays within compound", () => {
      const blocks = compoundBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("c1");
      sel.move("down");
      expect(sel.selectedIDs.value).toEqual(["c2"]);

      sel.move("down");
      expect(sel.selectedIDs.value).toEqual(["c3"]);
    });

    it("move at compound boundary pops to compound parent", () => {
      const blocks = compoundBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("c3");
      sel.move("down"); // at bottom of compound → pop to bq
      expect(sel.selectedIDs.value).toEqual(["bq"]);
    });

    it("top-level range skips compound children and includes compound itself", () => {
      const blocks = compoundBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p1"); // anchor at top level
      sel.range("p1", "p2");

      // Should include p1, bq, p2 — not c1, c2, c3
      expect(sel.isSelected("p1")).toBe(true);
      expect(sel.isSelected("bq")).toBe(true);
      expect(sel.isSelected("p2")).toBe(true);
      expect(sel.isSelected("c1")).toBe(false);
    });
  });

  describe("focus watcher clears selection", () => {
    it("clears selection when focus changes to an unselected block", async () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p1");
      sel.toggle("p2");
      expect(sel.selectedIDs.value).toHaveLength(2);

      focus.focusBlock("p4"); // p4 is not in the selection
      await nextTick(); // watcher fires on nextTick

      expect(sel.selectedIDs.value).toHaveLength(0);
    });

    it("does not clear selection when focusing an already-selected block", async () => {
      const blocks = flatBlocks();
      const focus = useBlockFocus(blocks);
      const sel = useBlockSelection(blocks, focus);

      sel.select("p1");
      sel.toggle("p2");

      focus.focusBlock("p1"); // p1 is already selected
      await nextTick();

      expect(sel.selectedIDs.value).toHaveLength(2);
    });
  });
});
