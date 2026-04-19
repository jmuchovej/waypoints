<script lang="ts">
import type { RenderNode } from "@waypoints/editor";

export interface BlockProps {
  block: RenderNode;
  /** Sequential index for enum-item numbering */
  enumIndex?: number;
}
</script>

<script setup lang="ts">
import { createReusableTemplate } from "@vueuse/core";
import { BlockOrchestratorKey, type OrchestratorContext, type OrderPosition } from "../composables";
import { registry, ariaSemantics } from "./blocks";
import { isClosestTargetMark } from "../utils/elements";
import { isFocusableType, isLandmark, isContent } from "@waypoints/domain";

const props = defineProps<BlockProps>();

const orchestrator = inject<OrchestratorContext>(BlockOrchestratorKey);
const { focus, dragDrop, selection } = orchestrator;

const [DefineDropIndicator, DropIndicator] = createReusableTemplate<{
  position: OrderPosition;
}>();

/** Compound parents inject their own depth so children indent relative to the container. */
const parentDepthOverride = inject<number>("compound-parent-depth", -1);

const isSelected = computed(() => selection.isSelected(props.block.id));
const isDragging = computed(() => dragDrop?.isDragging(props.block.id));
const isInsideDropTarget = computed(
  () => dragDrop?.isTarget(props.block.id) && dragDrop?.isInside(),
);

/**
 * Intercept pointerdown with Shift/Cmd to handle block selection before
 * ProseMirror can grab focus.
 */
function onPointerDown(e: PointerEvent) {
  const hasModifier = e.shiftKey || e.metaKey || e.ctrlKey;
  if (!hasModifier) return;

  // Don't interfere with clicks on child blocks inside compounds
  if (isClosestTargetMark(e, props.block.id)) return;

  e.preventDefault();
  e.stopPropagation();

  if (e.shiftKey) {
    selection.shiftClick(props.block.id);
  } else {
    // Cmd/Ctrl+Click
    selection.toggle(props.block.id);
  }

  focus.blurAll();
}

function onClick(e: MouseEvent) {
  // Modifier clicks are handled by onPointerDown
  if (e.shiftKey || e.metaKey || e.ctrlKey) return;

  // Blocks with a live editor handle: let clicks pass through to the editor
  if (focus?.getHandle(props.block.id)) return;

  // Don't interfere with clicks on child blocks
  if (isClosestTargetMark(e, props.block.id)) return;

  // Text-editable blocks without a handle yet: focus to start editing
  const type = props.block.data.mark === "content" ? props.block.data.type : null;
  if (type && isFocusableType(type)) {
    focus?.focusBlockAt(props.block.id, e);
    return;
  }

  // Compound/non-editable blocks: select
  focus?.blurAll();
  selection?.select(props.block.id);
}

const effectiveDepth = computed(() => {
  if (parentDepthOverride >= 0) {
    return props.block.depth - parentDepthOverride - 1;
  }
  return props.block.depth;
});

const depth = computed(() => {
  return Math.max(0, effectiveDepth.value);
});

const blockType = computed(() => {
  const { data } = props.block;
  if (isLandmark(data)) return "landmark";
  if (isContent(data)) return data.type;
  return undefined;
});
</script>

<template>
  <DefineDropIndicator v-slot="{ position }">
    <div
      class="drop-indicator"
      aria-hidden="true"
      v-if="dragDrop.showDropTarget(block.id, position)"
    />
  </DefineDropIndicator>
  <div class="block">
    <DropIndicator position="before" />
    <div
      class="mark"
      :style="`--depth: ${depth};`"
      :class="{
        selected: isSelected,
        dragging: isDragging,
        'inside-drop-target': isInsideDropTarget,
      }"
      :data-mark-id="block.id"
      :data-depth="block.depth"
      :data-type="blockType"
      v-bind="ariaSemantics(block.data)"
      @pointerdown.capture="onPointerDown"
      @click="onClick"
    >
      <component :is="registry(block.data)" :block="block" :enum-index="enumIndex" />
    </div>
    <DropIndicator position="after" />
  </div>
</template>
