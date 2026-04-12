<script lang="ts">
export interface FloatingHandlesProps {
  container: HTMLDivElement | null;
}

import { isInGutter, handlePosition } from "../utils/floating-handle";
import { getMarkID, getClosestBlock } from "../utils/elements";

function positionFor(markEl: HTMLElement) {
  const pos = handlePosition(markEl, props.container!);
  handleTop.value = pos.top;
  handleLeft.value = pos.left;
  hoveredMarkId.value = getMarkID(markEl);
  isVisible.value = true;
}
</script>

<script setup lang="ts">
import { BlockOrchestratorKey, type OrchestratorContext } from "../composables";

const props = defineProps<FloatingHandlesProps>();

const orchestrator = inject<OrchestratorContext>(BlockOrchestratorKey);
const { dragDrop } = orchestrator;

const hoveredMarkId = ref<MarkID | null>(null);
const handleTop = ref(0);
const handleLeft = ref(0);
const isVisible = ref(false);

// Non-reactive dedup ref — avoids recalculating when the cursor stays on the same block
let lastMarkEl: HTMLElement | null = null;

const shouldShow = computed(() => isVisible.value && dragDrop.blockIDs.value.length === 0);

function onContainerPointerMove(event: PointerEvent) {
  if (dragDrop.blockIDs.value.length > 0) return;

  const el = getClosestBlock(event);
  if (el) {
    if (el !== lastMarkEl) {
      // If the new target is an ancestor of the current one (e.g., compound
      // parent wrapping a child block), keep the handles on the child while
      // the cursor is still vertically within the child's bounds.
      if (lastMarkEl && el.contains(lastMarkEl)) {
        const { top, bottom } = lastMarkEl.getBoundingClientRect();
        const { clientY } = event;
        if (clientY >= top && clientY <= bottom) return;
      }
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

function onHandlePointerDown(e: PointerEvent) {
  if (!hoveredMarkId.value) return;
  const { dragDrop, selection } = orchestrator;
  const ids = selection.isSelected(hoveredMarkId.value)
    ? sortByVisualOrder([...selection.selectedIDs.value])
    : [hoveredMarkId.value];
  dragDrop.onPointerDown(ids, e);
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
    @pointerdown.stop="onHandlePointerDown"
  >
    <div class="floating-handle handle-add">
      <UIcon name="i-lucide-plus" />
    </div>
    <div class="floating-handle handle-drag">
      <UIcon name="i-lucide-grip-vertical" />
    </div>
  </div>
</template>
