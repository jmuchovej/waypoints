import { isCompoundType, isNestableType, isIndentableType, type MarkID } from "@waypoints/domain";
import type { RenderNode } from "@waypoints/editor";
import { readonly, ref, type Ref } from "vue";
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

/** Pixels per depth level: 6 spacing units × 4px (0.25rem × 16px default). */
export const INDENT_UNIT_PX = 24;

export type MoveBlocksFn = (
  srcIDs: MarkID[],
  dstID: MarkID,
  pos: BlockPosition,
  dropDepth: number,
) => Promise<void>;

/**
 * Compute the drop depth from cursor X position.
 * Rounds to nearest valid depth, clamped to [minDepth, maxDepth].
 */
export function computeDropDepth(
  cursorX: number,
  containerLeft: number,
  maxDepth: number,
  minDepth: number,
): number {
  const relativeX = cursorX - containerLeft;
  const raw = Math.round(relativeX / INDENT_UNIT_PX);
  return Math.max(minDepth, Math.min(maxDepth, raw));
}

export function useBlockDragDrop(
  blocks: Ref<RenderNode[]>,
  containerEl: Ref<HTMLElement | null>,
  doMove: MoveBlocksFn,
) {
  const blockIDs = ref<MarkID[]>([]);
  const targetID = ref<MarkID | null>(null);
  const position = ref<BlockPosition>("after");
  const dropDepth = ref<number>(0);

  function isCompound(id: MarkID): boolean {
    const block = blocks.value.find((b) => b.id === id);
    if (!block) return false;
    return isCompoundType(block.data);
  }

  function isEmptyCompound(id: MarkID): boolean {
    return isCompound(id) && !blocks.value.some((b) => b.parent_id === id);
  }

  /** Get the last child of a compound block (for resolving "after last child" targets). */
  function lastChildOf(compoundID: MarkID): MarkID | null {
    const children = blocks.value.filter((b) => b.parent_id === compoundID);
    return children.length > 0 ? children[children.length - 1]!.id : null;
  }

  /** Find the minimum depth for a block within its compound scope. */
  function compoundScopeMinDepth(blockId: MarkID): number {
    let current = blocks.value.find((b) => b.id === blockId);
    while (current) {
      const parent = blocks.value.find((b) => b.id === current!.parent_id);
      if (parent && isCompoundType(parent.data)) {
        return parent.depth + 1;
      }
      current = parent;
    }
    return 0;
  }

  /** Check if a block is a descendant of any dragged block (prevents dropping into self). */
  function isDescendantOfDragged(id: MarkID): boolean {
    let current = blocks.value.find((b) => b.id === id);
    while (current) {
      if (draggedSet.has(current.parent_id as MarkID)) return true;
      current = blocks.value.find((b) => b.id === current!.parent_id);
    }
    return false;
  }

  // ── Ghost element (floating clone that follows the cursor) ──────────

  let ghostEl: HTMLElement | null = null;
  let ghostOffsetX = 0;
  let ghostOffsetY = 0;

  function createGhost(ids: MarkID[], e: PointerEvent) {
    if (ids.length < 1) return;

    const el0 = getBlock(ids[0] as MarkID);
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

  /** Check if moving the dragged blocks to (dstID, pos, depth) would be a no-op. */
  function isNoOpMove(srcIDs: MarkID[], dstID: MarkID, pos: OrderPosition, depth: number): boolean {
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
    let lastSrc = srcIndices[srcIndices.length - 1]!;

    // Expand lastSrc to include descendants of dragged blocks (e.g. compound children)
    // that occupy contiguous indices after the last dragged block.
    while (lastSrc + 1 < ids.length) {
      const nextId = ids[lastSrc + 1] as MarkID;
      const nextBlock = blocks.value.find((b) => b.id === nextId);
      if (nextBlock && isDescendantOfDragged(nextId)) {
        lastSrc++;
      } else {
        break;
      }
    }

    // Check positional adjacency
    const isAdjacent =
      (pos === "after" && dstIdx === firstSrc - 1) || (pos === "before" && dstIdx === lastSrc + 1);

    if (!isAdjacent) return false;

    // Different depth is a reparent — not a no-op
    const srcBlock = blocks.value.find((b) => b.id === srcIDs[0]);
    if (srcBlock && srcBlock.depth !== depth) return false;

    // Different parent is a reparent — not a no-op.
    // Exception: if the target IS the source's parent (e.g. compound block above its first child),
    // and depth matches, it's still a no-op.
    const dstBlock = blocks.value.find((b) => b.id === dstID);
    if (srcBlock && dstBlock) {
      const sameParent = srcBlock.parent_id === dstBlock.parent_id;
      const targetIsParent = srcBlock.parent_id === dstID;
      if (!sameParent && !targetIsParent) return false;
    }

    return true;
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
    if (draggedSet.has(target) || isDescendantOfDragged(target)) {
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
    // resolve to the nearest non-dragged child, or allow dragging out.
    if (isCompound(target)) {
      const children = blocks.value.filter((b) => b.parent_id === target);
      const nonDraggedChildren = children.filter((b) => !draggedSet.has(b.id));

      if (nonDraggedChildren.length > 0) {
        // Resolve to the nearest non-dragged child by Y
        const nearest = nonDraggedChildren.reduce(
          (best, child) => {
            const childEl = getBlock(child.id);
            if (!childEl) return best;
            const rect = childEl.getBoundingClientRect();
            const dist = Math.abs(e.clientY - (rect.top + rect.height / 2));
            if (!best.el) return { el: childEl, dist, block: child };
            return dist < best.dist ? { el: childEl, dist, block: child } : best;
          },
          { el: null, dist: Infinity, block: null } as {
            el: BlockElement | null;
            dist: number;
            block: RenderNode | null;
          },
        );

        if (nearest.block) {
          const childRect = nearest.el!.getBoundingClientRect();
          const childMidY = childRect.top + childRect.height / 2;
          const childPos = e.clientY < childMidY ? "before" : "after";
          const childDepth = nearest.block.depth;
          if (
            targetID.value !== nearest.block.id ||
            position.value !== childPos ||
            dropDepth.value !== childDepth
          ) {
            log.debug("drop target (compound → child)", {
              compoundId: target,
              childId: nearest.block.id,
              pos: childPos,
            });
            targetID.value = nearest.block.id;
            position.value = childPos;
            // Lock depth to the child's level — sibling placement within the compound
            dropDepth.value = childDepth;
          }
          return;
        }
      } else if (children.length > 0) {
        // All children are being dragged — allow dropping after the compound to drag out.
        // Only when cursor is in the lower half.
        const rect = blockEl.getBoundingClientRect();
        const midY = rect.top + rect.height / 2;
        if (e.clientY > midY) {
          const compoundDepth = blocks.value.find((b) => b.id === target)?.depth ?? 0;
          if (
            targetID.value !== target ||
            position.value !== "after" ||
            dropDepth.value !== compoundDepth
          ) {
            log.debug("drop target (compound, dragging children out)", { compoundId: target });
            targetID.value = target;
            position.value = "after";
            dropDepth.value = compoundDepth;
          }
          return;
        }
        // Upper half — suppress indicator (no-op, can't reorder with yourself)
        if (targetID.value) targetID.value = null;
        return;
      } else {
        // Empty compound — handled by isEmptyCompound above, shouldn't reach here
        return;
      }
    }

    const rect = blockEl.getBoundingClientRect();
    const midY = rect.top + rect.height / 2;
    const pos = e.clientY < midY ? "before" : "after";

    // ── Compute drop depth from cursor X ──────────────────────────────
    const targetBlock = blocks.value.find((b) => b.id === target);
    const flatIdx = blocks.value.findIndex((b) => b.id === target);

    // Preceding block: for "after" it's the target itself; for "before" it's the block above.
    // If the preceding block is deeper than the target (e.g. last child of a compound above),
    // walk up to find the actual preceding block at or above the target's depth.
    let preceding =
      pos === "after" ? targetBlock : flatIdx > 0 ? blocks.value[flatIdx - 1] : undefined;

    if (preceding && targetBlock && preceding.depth > targetBlock.depth) {
      // The preceding block is inside a compound that the target is outside of.
      // Walk up the flat list to find the nearest block at or above target depth.
      // Stop if we reach a dragged block — anything above it is not a valid preceding block.
      preceding = undefined;
      for (let i = pos === "after" ? flatIdx : flatIdx - 1; i >= 0; i--) {
        const b = blocks.value[i]!;
        if (draggedSet.has(b.id) || isDescendantOfDragged(b.id)) break;
        if (b.depth <= targetBlock.depth) {
          preceding = b;
          break;
        }
      }
    }

    // Indentation requires: the preceding block is nestable (can accept children),
    // the dragged blocks are indentable (headings can't be indented), and the
    // preceding block is not being dragged itself.
    const srcBlock = blocks.value.find((b) => b.id === blockIDs.value[0]);
    const draggedIsIndentable = srcBlock ? isIndentableType(srcBlock.data) : true;
    const canNestUnder =
      preceding && draggedIsIndentable
        ? isNestableType(preceding.data) &&
          !draggedSet.has(preceding.id) &&
          !isDescendantOfDragged(preceding.id)
        : false;
    const precedingDepth = preceding?.depth ?? -1;
    const maxDepth = canNestUnder ? precedingDepth + 1 : precedingDepth;
    const minDepth = compoundScopeMinDepth(target);
    const containerRect = containerEl.value?.getBoundingClientRect();
    const containerLeft = containerRect?.left ?? 0;
    const depth = computeDropDepth(e.clientX, containerLeft, maxDepth, minDepth);

    // Suppress indicator if the move wouldn't change order or depth
    if (isNoOpMove(blockIDs.value, target, pos, depth)) {
      if (targetID.value) targetID.value = null;
      return;
    }

    if (targetID.value !== target || position.value !== pos || dropDepth.value !== depth) {
      log.debug("drop target", { targetId: target, pos, depth });
      targetID.value = target;
      position.value = pos;
      dropDepth.value = depth;
    }
  }

  async function onPointerUp(_e: PointerEvent) {
    document.removeEventListener("pointermove", onPointerMove);
    document.removeEventListener("pointerup", onPointerUp);

    const srcIDs = blockIDs.value;
    const dstID = targetID.value;
    const depth = dropDepth.value;

    log.debug("pointerup", { srcIDs, dstID, position: position.value, depth });

    cleanup();

    if (srcIDs.length === 0 || !dstID || srcIDs.includes(dstID)) return;

    await doMove(srcIDs, dstID, position.value, depth);
  }

  function cleanup() {
    removeGhost();
    blockIDs.value = [];
    targetID.value = null;
    dropDepth.value = 0;
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
    dropDepth: readonly(dropDepth),
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
