import { isCompoundType, type MarkID } from "@waypoints/domain";
import type { RenderNode } from "@waypoints/editor";
import { readonly, ref } from "vue";
import { createLogger } from "~/utils/logger";
import {
  getBlock,
  getBlocks,
  getClosestBlock,
  getMarkID,
  type BlockElement,
} from "../utils/elements";

const log = createLogger("drag");

export type BlockPosition = "before" | "after" | "inside";
export type OrderPosition = Exclude<BlockPosition, "inside">;
const MAX_VISIBLE = 5;

export type MoveBlocksFn = (srcIDs: MarkID[], dstID: MarkID, pos: BlockPosition) => Promise<void>;

export function useBlockDragDrop(blocks: Ref<RenderNode[]>, doMove: MoveBlocksFn) {
  const blockIDs = ref<MarkID[]>([]);
  const targetID = ref<MarkID | null>(null);
  const position = ref<BlockPosition>("after");

  function isCompound(id: MarkID): boolean {
    const block = blocks.value.find((b) => b.id === id);
    if (!block || isCompoundType(block.data)) return false;
    return true;
  }

  function isEmptyCompound(id: MarkID): boolean {
    return isCompound(id) && !blocks.value.some((b) => b.parent_id === id);
  }

  /** Get the last child of a compound block (for resolving "after last child" targets). */
  function lastChildOf(compoundID: MarkID): MarkID | null {
    const children = blocks.value.filter((b) => b.parent_id === compoundID);
    return children.length > 0 ? children[children.length - 1]!.id : null;
  }

  // ── Ghost element (floating clone that follows the cursor) ──────────

  let ghostEl: HTMLElement | null = null;
  let ghostOffsetX = 0;
  let ghostOffsetY = 0;

  function createGhost(ids: MarkID[], e: PointerEvent) {
    if (ids.length < 1) return;

    const el0 = getBlock(ids[0]);
    if (!el0) return;

    const rect = el0.getBoundingClientRect();
    ghostOffsetX = e.clientX - rect.left;
    ghostOffsetY = e.clientY - rect.top;

    if (ids.length === 1) {
      // Single block — clone the element directly (existing behavior)
      ghostEl = el0.cloneNode(true) as HTMLElement;
      ghostEl.className = "drag-ghost";
    } else {
      // Multi-block — stacked preview
      ghostEl = document.createElement("div");
      ghostEl.className = "drag-ghost multi";

      const visible = ids.slice(0, MAX_VISIBLE);
      for (const id of visible) {
        const srcEl = getBlock(id);
        if (!srcEl) continue;
        const item = srcEl.cloneNode(true) as BlockElement;
        item.className = "drag-ghost-item";
        ghostEl.appendChild(item);
      }

      if (ids.length > MAX_VISIBLE) {
        const overflow = document.createElement("div");
        overflow.className = "drag-ghost-overflow";
        overflow.textContent = `+${ids.length - MAX_VISIBLE} more`;
        ghostEl.appendChild(overflow);
      }
    }

    ghostEl.style.left = `${rect.left}px`;
    ghostEl.style.top = `${rect.top}px`;
    ghostEl.style.width = `${rect.width}px`;

    document.body.appendChild(ghostEl);

    // Trigger the lift animation on the next frame
    requestAnimationFrame(() => ghostEl?.classList.add("lifting"));
  }

  function moveGhost(e: PointerEvent) {
    if (!ghostEl) return;
    ghostEl.style.left = `${e.clientX - ghostOffsetX}px`;
    ghostEl.style.top = `${e.clientY - ghostOffsetY}px`;
  }

  function removeGhost() {
    if (ghostEl) {
      ghostEl.remove();
      ghostEl = null;
    }
  }

  // ── Pointer event handlers ──────────────────────────────────────────

  function onPointerDown(ids: MarkID[], e: PointerEvent) {
    if (e.button !== 0) return;

    e.preventDefault();

    log.debug("pointerdown", ids);
    blockIDs.value = ids;

    createGhost(ids, e);

    document.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerup", onPointerUp);
  }

  // O(1) lookup for dragged blocks
  let draggedSet: Set<MarkID> = new Set();

  /** Find the block element under the cursor, or the nearest block by Y if in margins. */
  function findBlockAt(x: number, y: number): BlockElement | null {
    const el = document.elementFromPoint(x, y) as HTMLElement | null;
    if (!el) return null;
    const direct = getClosestBlock(el);
    if (direct && !draggedSet.has(getMarkID(direct))) return direct;

    // Cursor is in margins/outside blocks — find the nearest non-dragged block by Y
    let nearest: BlockElement | null = null;
    let nearestDist = Infinity;

    for (const block of getBlocks()) {
      const id = getMarkID(block);
      if (draggedSet.has(id)) continue;
      const rect = block.getBoundingClientRect();
      const dist = y < rect.top ? rect.top - y : y > rect.bottom ? y - rect.bottom : 0;
      if (dist < nearestDist) {
        nearestDist = dist;
        nearest = block;
      }
    }

    return nearest;
  }

  /** Check if moving the dragged blocks to (dstID, pos) would keep them in the same BlockPosition. */
  function isNoOpMove(srcIDs: MarkID[], dstID: MarkID, pos: OrderPosition): boolean {
    const els = getBlocks();
    const ids = Array.from(els).map((el) => el.dataset.markId);

    // For non-contiguous selections, always allow (consolidation is structural)
    const srcIndices = srcIDs
      .map((id) => ids.indexOf(id))
      .filter((i) => i >= 0)
      .sort((a, b) => a - b);
    const isContiguous = srcIndices.every((idx, i) => i === 0 || idx === srcIndices[i - 1]! + 1);
    if (!isContiguous) return false;

    const dstIdx = ids.indexOf(dstID);
    if (dstIdx < 0 || srcIndices.length === 0) return false;

    const firstSrc = srcIndices[0]!;
    const lastSrc = srcIndices[srcIndices.length - 1]!;

    // "after" the block right before the group = same BlockPosition
    if (pos === "after" && dstIdx === firstSrc - 1) return true;
    // "before" the block right after the group = same BlockPosition
    if (pos === "before" && dstIdx === lastSrc + 1) return true;

    return false;
  }

  function onPointerMove(e: PointerEvent) {
    if (blockIDs.value.length === 0) return;

    moveGhost(e);

    // Refresh the dragged set (in case it changed, though it shouldn't mid-drag)
    draggedSet = new Set(blockIDs.value);

    const blockEl = findBlockAt(e.clientX, e.clientY);

    if (!blockEl) {
      if (targetID.value) {
        log.debug("cleared drop target (outside editor)");
        targetID.value = null;
      }
      return;
    }

    const target = blockEl.getAttribute("data-mark-id") as MarkID;
    if (draggedSet.has(target)) {
      if (targetID.value) {
        targetID.value = null;
      }
      return;
    }

    // Empty compound: always "inside" (dashed outline)
    if (isEmptyCompound(target)) {
      if (targetID.value !== target || position.value !== "inside") {
        log.debug("drop target (inside empty)", { targetId: target });
        targetID.value = target;
        position.value = "inside";
      }
      return;
    }

    // Non-empty compound hit directly (cursor on padding/accent bar, not a child):
    // resolve to last child "after" so a line indicator renders inside the compound.
    if (isCompound(target)) {
      const lastChild = lastChildOf(target);
      if (lastChild) {
        if (targetID.value !== lastChild || position.value !== "after") {
          log.debug("drop target (compound → last child)", { compoundId: target, lastChild });
          targetID.value = lastChild;
          position.value = "after";
        }
        return;
      }
    }

    const rect = blockEl.getBoundingClientRect();
    const midY = rect.top + rect.height / 2;
    const pos = e.clientY < midY ? "before" : "after";

    // Suppress indicator if the move wouldn't change order
    if (isNoOpMove(blockIDs.value, target, pos)) {
      if (targetID.value) targetID.value = null;
      return;
    }

    if (targetID.value !== target || position.value !== pos) {
      log.debug("drop target", { targetId: target, pos });
      targetID.value = target;
      position.value = pos;
    }
  }

  async function onPointerUp(_e: PointerEvent) {
    document.removeEventListener("pointermove", onPointerMove);
    document.removeEventListener("pointerup", onPointerUp);

    const srcIDs = blockIDs.value;
    const dstID = targetID.value;

    log.debug("pointerup", { srcIDs, dstID, BlockPosition: position.value });

    cleanup();

    if (srcIDs.length === 0 || !dstID || srcIDs.includes(dstID)) return;

    await doMove(srcIDs, dstID, position.value);
  }

  function cleanup() {
    removeGhost();
    blockIDs.value = [];
    targetID.value = null;
    draggedSet = new Set();
  }

  function isDragging(id: undefined): boolean;
  function isDragging(id: MarkID | undefined): boolean {
    if (id === undefined) return blockIDs.value.length > 0;

    return blockIDs.value.includes(id);
  }

  function isTarget(id: MarkID): boolean {
    return targetID.value === id;
  }

  function getBlockPosition(): BlockPosition {
    return position.value;
  }

  function isInside(): boolean {
    return getBlockPosition() === "inside";
  }
  function isBefore(): boolean {
    return getBlockPosition() === "before";
  }
  function isAfter(): boolean {
    return getBlockPosition() === "after";
  }
  function showDropTarget(id: MarkID, pos: OrderPosition): boolean {
    return blockIDs.value.length > 0 && isTarget(id) && getBlockPosition() === pos;
  }
  return {
    blockIDs: readonly(blockIDs),
    targetID: readonly(targetID),
    BlockPosition: readonly(position),
    onPointerDown,
    isDragging,
    isTarget,
    getBlockPosition,
    isInside,
    isBefore,
    isAfter,
    showDropTarget,
  };
}

export type DragDropContext = ReturnType<typeof useBlockDragDrop>;
