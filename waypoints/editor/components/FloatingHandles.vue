<script lang="ts">
export interface FloatingHandlesProps {
  container: HTMLDivElement | null;
}

import { isInGutter, handlePosition } from "../utils/floating-handle";
import { getMarkID, getClosestBlock, type BlockElement } from "../utils/elements";
import BlockContextMenu from "./BlockContextMenu.vue";
import type { MarkID } from "@waypoints/domain";
</script>

<script setup lang="ts">
import { BlockOrchestratorKey, type OrchestratorContext } from "../composables";

const props = defineProps<FloatingHandlesProps>();

const orchestrator = inject<OrchestratorContext>(BlockOrchestratorKey)!;
const { dragDrop } = orchestrator;

const hoveredMarkId = ref<MarkID | null>(null);
const handleTop = ref(0);
const handleLeft = ref(0);
const isVisible = ref(false);

function positionFor(markEl: HTMLElement) {
  const pos = handlePosition(markEl, props.container!);
  handleTop.value = pos.top;
  handleLeft.value = pos.left;
  hoveredMarkId.value = getMarkID(markEl);
  isVisible.value = true;
}

// Non-reactive dedup ref — avoids recalculating when the cursor stays on the same block
let lastMarkEl: HTMLElement | null = null;

const shouldShow = computed(
  () => (isVisible.value || contextMenuOpen.value) && dragDrop.blockIDs.value.length === 0,
);

function onContainerPointerMove(event: PointerEvent) {
  if (dragDrop.blockIDs.value.length > 0) return;

  let el = getClosestBlock(event);
  if (el) {
    // If the resolved block is inside a compound, check whether the cursor
    // is in the compound's "own" zone (left of the child's indent area).
    // If so, prefer the compound block for the drag handle.
    const parent = el.parentElement?.closest<BlockElement>(`[data-mark-id]`);
    if (parent) {
      const childLeft = el.getBoundingClientRect().left;
      if (event.clientX < childLeft) {
        el = parent;
      }
    }

    if (el !== lastMarkEl) {
      lastMarkEl = el;
      positionFor(el);
    }
    return;
  }

  // Inside the container but not over a block — check gutter
  if (lastMarkEl && isInGutter(lastMarkEl, event)) return;

  lastMarkEl = null;
  isVisible.value = false;
}

/** Global pointermove — keeps the handle alive when the cursor is in the gutter outside the container. */
function onDocumentPointerMove(event: PointerEvent) {
  if (!isVisible.value || !lastMarkEl) return;

  // If the cursor is back inside the container, onContainerPointerMove handles it
  if (props.container?.contains(event.target as Node)) return;

  if (isInGutter(lastMarkEl, event)) return;

  lastMarkEl = null;
  isVisible.value = false;
}

const DRAG_THRESHOLD_PX = 5;
let dragStartX = 0;
let dragStartY = 0;
let pendingDragEvent: PointerEvent | null = null;
let dragStarted = false;

function onDragPointerDown(e: PointerEvent) {
  if (!hoveredMarkId.value || e.button !== 0) return;

  dragStartX = e.clientX;
  dragStartY = e.clientY;
  pendingDragEvent = e;
  dragStarted = false;

  document.addEventListener("pointermove", onDragThresholdMove);
  document.addEventListener("pointerup", onDragThresholdUp);
}

function onDragThresholdMove(e: PointerEvent) {
  const dx = e.clientX - dragStartX;
  const dy = e.clientY - dragStartY;
  if (Math.abs(dx) + Math.abs(dy) >= DRAG_THRESHOLD_PX && pendingDragEvent) {
    dragStarted = true;
    contextMenuOpen.value = false;
    e.preventDefault();
    document.removeEventListener("pointermove", onDragThresholdMove);
    document.removeEventListener("pointerup", onDragThresholdUp);

    const { dragDrop, selection } = orchestrator;
    const ids = selection.isSelected(hoveredMarkId.value!)
      ? sortByVisualOrder([...selection.selectedIDs.value])
      : [hoveredMarkId.value!];
    dragDrop.onPointerDown(ids, pendingDragEvent);
    pendingDragEvent = null;
  }
}

function onDragThresholdUp(_e: PointerEvent) {
  document.removeEventListener("pointermove", onDragThresholdMove);
  document.removeEventListener("pointerup", onDragThresholdUp);
  pendingDragEvent = null;

  // If no drag started, the UDropdownMenu trigger handles the click natively.
}

const contextMenuOpen = ref(false);

function onAddClick(e: MouseEvent) {
  if (!hoveredMarkId.value) return;
  if (e.altKey) {
    // Alt+click: insert block above
    orchestrator.onCreateBlockBefore(hoveredMarkId.value);
  } else {
    // Click: insert block below
    orchestrator.onCreateBlockAfter(hoveredMarkId.value);
  }
}

/** Sort IDs into the order they appear in the rendered block list. */
function sortByVisualOrder(ids: string[]): string[] {
  const idSet = new Set(ids);
  return blocks.value.filter((b) => idSet.has(b.id)).map((b) => b.id);
}

const blocks = orchestrator.blocks;

watchEffect((onCleanup) => {
  const el = props.container;
  if (!el) return;

  el.addEventListener("pointermove", onContainerPointerMove, { passive: true });
  document.addEventListener("pointermove", onDocumentPointerMove, {
    passive: true,
  });

  onCleanup(() => {
    el.removeEventListener("pointermove", onContainerPointerMove);
    document.removeEventListener("pointermove", onDocumentPointerMove);
  });
});
</script>

<template>
  <div
    class="floating-handles"
    :class="{ 'opacity-0 pointer-events-none': !shouldShow }"
    :style="{
      top: `${handleTop}px`,
      left: `${handleLeft}px`,
    }"
  >
    <div class="floating-handle handle-add" @click.stop="onAddClick">
      <UIcon name="i-lucide-plus" />
    </div>
    <BlockContextMenu v-if="hoveredMarkId" v-model:open="contextMenuOpen" :block-id="hoveredMarkId">
      <div class="floating-handle handle-drag" @pointerdown.stop="onDragPointerDown">
        <UIcon name="i-lucide-grip-vertical" />
      </div>
    </BlockContextMenu>
    <div v-else class="floating-handle handle-drag" @pointerdown.stop="onDragPointerDown">
      <UIcon name="i-lucide-grip-vertical" />
    </div>
  </div>
</template>
