import { type MarkID, isCompoundType } from "@waypoints/domain";
import type { RenderNode } from "@waypoints/editor";
import type { Ref } from "vue";
import { computed, readonly, ref, watch } from "vue";
import { createLogger } from "~/utils/logger";
import type { FocusContext } from "./useBlockFocus";

const log = createLogger("selection");

export type SelectionDirection = "up" | "down";

export function useBlockSelection(blocks: Ref<RenderNode[]>, focus: FocusContext) {
  const selectedBlockIDs = ref<Set<MarkID>>(new Set());

  /** The anchor block for range selection (where Shift+Click started). */
  const anchorID = ref<MarkID | null>(null);

  const hasMultiSelect = computed(() => selectedBlockIDs.value.size > 1);
  const hasSingleSelect = computed(() => selectedBlockIDs.value.size >= 1);

  const selectedIDs = computed(() => [...selectedBlockIDs.value]);

  // ── Compound block helpers ──────────────────────────────────────────

  /** Find the compound parent of a block, if it's inside one. */
  function getCompoundParent(blockId: MarkID): RenderNode | null {
    const block = blocks.value.find((b) => b.id === blockId);
    if (!block?.parent_id) return null;
    const parent = blocks.value.find((b) => b.id === block.parent_id);
    if (parent && isCompoundType(parent.data)) return parent;
    return null;
  }

  /**
   * Get navigable block IDs based on the current scope.
   * - If anchor is inside a compound → siblings within that compound
   * - Otherwise → top-level blocks (compound children hidden, compounds included)
   */
  function getNavigableIDs(): MarkID[] {
    const anchor = anchorID.value;
    if (anchor) {
      const compoundParent = getCompoundParent(anchor);
      if (compoundParent) {
        return blocks.value.filter((b) => b.parent_id === compoundParent.id).map((b) => b.id);
      }
    }
    // Top-level: include all blocks that aren't compound children
    return blocks.value
      .filter((b) => {
        if (!b.parent_id) return true;
        const parent = blocks.value.find((p) => p.id === b.parent_id);
        if (!parent) return true; // parent is the landmark root
        return !isCompoundType(parent.data);
      })
      .map((b) => b.id);
  }

  /** Pop selection out of a compound block to select the compound itself. */
  function popToCompound(): boolean {
    const firstSelected = [...selectedBlockIDs.value][0];
    if (!firstSelected) return false;
    const compoundParent = getCompoundParent(firstSelected);
    if (!compoundParent) return false;
    log.debug("popToCompound", { from: firstSelected, to: compoundParent.id });
    selectedBlockIDs.value = new Set([compoundParent.id]);
    anchorID.value = compoundParent.id;
    return true;
  }

  // ── Selection operations ────────────────────────────────────────────

  function clear() {
    selectedBlockIDs.value = new Set();
    anchorID.value = null;
  }

  /** Select a contiguous range between two block IDs (inclusive). */
  function range(srcID: MarkID, dstID: MarkID) {
    const ids = getNavigableIDs();
    const srcIdx = ids.indexOf(srcID);
    const dstIdx = ids.indexOf(dstID);
    if (srcIdx < 0 || dstIdx < 0) return;

    const min = Math.min(srcIdx, dstIdx);
    const max = Math.max(srcIdx, dstIdx);

    const selections = new Set<MarkID>();
    for (let i = min; i <= max; i++) {
      selections.add(ids[i]!);
    }
    selectedBlockIDs.value = selections;
  }

  /** Toggle a single block in/out of the selection (Ctrl/Cmd+Click). */
  function toggle(id: MarkID) {
    const next = new Set(selectedBlockIDs.value);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedBlockIDs.value = next;
  }

  /** Shift+Click handler: select range from anchor (or focused block) to target. */
  function shiftClick(targetID: MarkID) {
    const anchor = anchorID.value ?? focus.blockID.value;
    if (!anchor) {
      // Nothing to range from — just select the target
      selectedBlockIDs.value = new Set([targetID]);
      anchorID.value = targetID;
      return;
    }
    range(anchor, targetID);
  }

  /** Extend selection by one block in the given direction (Shift+Arrow). */
  function extend(direction: SelectionDirection) {
    const ids = getNavigableIDs();

    // Determine the edge of current selection
    if (selectedBlockIDs.value.size === 0) {
      const focused = focus.blockID.value;
      if (!focused) return;
      // Start selection from focused block
      anchorID.value = focused;
      selectedBlockIDs.value = new Set([focused]);
    }

    // Find the furthest selected block in the direction we're extending
    const selectedIndices = [...selectedBlockIDs.value]
      .map((id) => ids.indexOf(id))
      .filter((i) => i >= 0)
      .sort((a, b) => a - b);

    if (selectedIndices.length === 0) return;

    const anchor = anchorID.value ? ids.indexOf(anchorID.value) : selectedIndices[0]!;

    log.debug("extend", {
      direction,
      anchorIdx: anchor,
      selectedIndices,
      navigableCount: ids.length,
      selectionSize: selectedBlockIDs.value.size,
    });

    if (direction === "up") {
      const topIdx = selectedIndices[0]!;
      const bottomIdx = selectedIndices[selectedIndices.length - 1]!;

      if (bottomIdx > anchor && bottomIdx > topIdx) {
        // Shrink from bottom
        log.debug("shrink from bottom", { bottomIdx, removing: ids[bottomIdx] });
        const next = new Set(selectedBlockIDs.value);
        next.delete(ids[bottomIdx]!);
        selectedBlockIDs.value = next;
      } else if (topIdx > 0) {
        // Extend upward
        log.debug("extend upward", { topIdx, adding: ids[topIdx - 1] });
        const next = new Set(selectedBlockIDs.value);
        next.add(ids[topIdx - 1]!);
        selectedBlockIDs.value = next;
      } else {
        // At boundary — pop out of compound if inside one
        log.debug("at boundary (up)");
        popToCompound();
      }
    } else {
      const minIdx = selectedIndices[0]!;
      const maxIdx = selectedIndices[selectedIndices.length - 1]!;

      if (minIdx < anchor && minIdx < maxIdx) {
        // Shrink from top
        log.debug("shrink from top", { minIdx, removing: ids[minIdx] });
        const next = new Set(selectedBlockIDs.value);
        next.delete(ids[minIdx]!);
        selectedBlockIDs.value = next;
      } else if (maxIdx < ids.length - 1) {
        // Extend downward
        log.debug("extend downward", { maxIdx, adding: ids[maxIdx + 1] });
        const next = new Set(selectedBlockIDs.value);
        next.add(ids[maxIdx + 1]!);
        selectedBlockIDs.value = next;
      } else {
        // At boundary — pop out of compound if inside one
        log.debug("at boundary (down)");
        popToCompound();
      }
    }

    log.debug("selection after", {
      size: selectedBlockIDs.value.size,
      ids: [...selectedBlockIDs.value],
    });
  }

  /** Select a single block (e.g. after Escape from editing). */
  function select(id: MarkID) {
    selectedBlockIDs.value = new Set([id]);
    anchorID.value = id;
  }

  /** Move single-block selection to the adjacent block in the given direction. */
  function move(direction: SelectionDirection) {
    const ids = getNavigableIDs();
    const selected = [...selectedBlockIDs.value];
    if (selected.length === 0) return;

    const currentId = selected[0]!;
    const currentIdx = ids.indexOf(currentId);
    if (currentIdx < 0) return;

    if (direction === "up" && currentIdx > 0) {
      const prevId = ids[currentIdx - 1]!;
      selectedBlockIDs.value = new Set([prevId]);
      anchorID.value = prevId;
    } else if (direction === "down" && currentIdx < ids.length - 1) {
      const nextId = ids[currentIdx + 1]!;
      selectedBlockIDs.value = new Set([nextId]);
      anchorID.value = nextId;
    } else {
      // At boundary — try to pop out of compound
      popToCompound();
    }
  }

  function isSelected(id: MarkID): boolean {
    return selectedBlockIDs.value.has(id);
  }

  // Clear selection when focus changes to a single block without shift held
  watch(focus.blockID, (newId) => {
    if (newId && !selectedBlockIDs.value.has(newId)) {
      log.debug("watcher clearing selection", {
        newFocusId: newId,
        selectedIds: [...selectedBlockIDs.value],
      });
      clear();
    }
  });

  return {
    selectedBlockIDs: readonly(selectedBlockIDs),
    selectedIDs,
    hasSingleSelect,
    hasMultiSelect,
    anchorID: readonly(anchorID),
    clear,
    select,
    range,
    toggle,
    shiftClick,
    extend,
    move,
    isSelected,
  };
}

export type SelectionContext = ReturnType<typeof useBlockSelection>;
