<script lang="ts">
import type { MarkID } from "@waypoints/domain";
export interface DocumentBodyProps {
  landmarkId: MarkID;
  onExitTop?: () => void;
}
</script>

<script setup lang="ts">
import type { RenderNode } from "@waypoints/editor";
import ipc from "~/ipc";
import { listen } from "@tauri-apps/api/event";
import {
  useBlockOrchestrator,
  useBlockKeymap,
  useCommandMenu,
  useLongPressDrag,
} from "../composables";
import { ContentType, isCompoundType, isContent } from "@waypoints/domain";
import { indexFor } from "../utils/list-item";
import Block from "./Block.vue";
import FloatingHandles from "./FloatingHandles.vue";
import SlashMenu from "./SlashMenu.vue";

const props = defineProps<DocumentBodyProps>();

const blocks = ref<RenderNode[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);

const landmarkIdRef = computed(() => props.landmarkId);
const commandMenu = useCommandMenu();
provide("command-menu", commandMenu);

const containerRef = ref<HTMLDivElement | null>(null);
const orchestrator = useBlockOrchestrator(landmarkIdRef, blocks, containerRef);
const { focus, selection, dragDrop } = orchestrator;

// Wire up external navigation callback
orchestrator.onExitTop = props.onExitTop;
const { handleKeydown: baseHandleKeydown } = useBlockKeymap(orchestrator);
const { pressing, pressMarkId } = useLongPressDrag(containerRef, dragDrop);
const slashMenuRef = ref<InstanceType<typeof SlashMenu> | null>(null);

// Listen for native menu undo/redo events (Cmd+Z / Cmd+Shift+Z are intercepted by
// the Tauri menu before they reach the web view's keydown handler).
const unlisteners: (() => void)[] = [];
onMounted(async () => {
  unlisteners.push(await listen("app:edit:undo", () => orchestrator.onUndo()));
  unlisteners.push(await listen("app:edit:redo", () => orchestrator.onRedo()));
});
onUnmounted(() => unlisteners.forEach((fn) => fn()));

// Toggle .long-press-active on the pressed mark element
watchEffect(() => {
  const id = pressMarkId.value;
  const active = pressing.value;
  const el = id
    ? containerRef.value?.querySelector<HTMLElement>(`.mark[data-mark-id="${id}"]`)
    : null;

  if (el && active) {
    el.classList.add("long-press-active");
  }

  // Clean up any stale highlights
  const prev = containerRef.value?.querySelector(".long-press-active");
  if (prev && prev !== el) {
    prev.classList.remove("long-press-active");
  }
  if (prev === el && !active) {
    prev?.classList.remove("long-press-active");
  }
});

// When entering selection mode (selection active, no block editing), focus the
// container so keyboard events still reach the @keydown.capture handler.
watch([selection.hasSingleSelect, focus.blockID], ([hasSelection, focusedId]) => {
  if (hasSelection && !focusedId) {
    nextTick(() => containerRef.value?.focus());
  }
});

function handleKeydown(e: KeyboardEvent) {
  // Let the slash menu consume arrow/enter/escape when open
  if (commandMenu.isOpen.value && commandMenu.handleKey(e)) {
    slashMenuRef.value?.handleKeydown(e);
    return;
  }
  baseHandleKeydown(e);
}

/** Filter out blocks that are descendants of a compound container (they render inside their ancestor). */
const topLevelBlocks = computed(() => {
  // Build a set of all block IDs that live inside a compound's subtree
  const insideCompound = new Set<string>();
  for (const b of blocks.value) {
    if (insideCompound.has(b.parent_id ?? "")) {
      // Parent is inside a compound → so is this block
      insideCompound.add(b.id);
      continue;
    }
    const parent = blocks.value.find((p) => p.id === b.parent_id);
    if (parent && isCompoundType(parent.data)) {
      insideCompound.add(b.id);
    }
  }
  return blocks.value.filter((b) => !insideCompound.has(b.id));
});

function enumIndexFor(block: RenderNode, index: number): number | undefined {
  return indexFor(block, index, blocks.value);
}

async function load() {
  loading.value = true;
  error.value = null;

  try {
    blocks.value = await ipc.editor.projectLandmark(props.landmarkId, null);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);

watch(() => props.landmarkId, load);

/** Click on document background blurs any active editor */
function onBackgroundClick(e: MouseEvent) {
  if (e.target === e.currentTarget) {
    focus.blurAll();
  }
}

/** Click in the overscroll area: focus last empty block or append a new one */
async function onOverscrollClick() {
  const all = blocks.value;

  if (all.length === 0) {
    await orchestrator.onCreateFirstBlock();
    return;
  }

  const last = all[all.length - 1]!;

  if (isContent(last.data, ContentType.Paragraph) && isEmpty(last.data)) {
    focus.focusBlock(last.id, "start");
  } else {
    await orchestrator.onCreateBlockAfter(last.id);
  }
}

defineExpose({
  blocks,
  loading,
  error,
  reload: load,
  orchestrator,
  commandMenu,
});
</script>

<template>
  <div
    ref="containerRef"
    class="editor-document editor-content"
    tabindex="-1"
    @click="onBackgroundClick"
    @keydown.capture="handleKeydown"
  >
    <div v-if="loading" class="flex items-center justify-center py-8">
      <UIcon name="i-lucide-loader" class="animate-spin size-6 text-dimmed" />
    </div>

    <div v-else-if="error" class="text-error text-sm p-4">
      {{ error }}
    </div>

    <template v-else>
      <Block
        v-for="(block, i) in topLevelBlocks"
        :key="block.id"
        :block="block"
        :enum-index="enumIndexFor(block, i)"
      />

      <div
        class="min-h-[25vh] max-h-[40vh] h-[32.5vh] cursor-text"
        @click.self="onOverscrollClick"
      />
    </template>

    <FloatingHandles :container="containerRef" />
    <SlashMenu ref="slashMenuRef" />
  </div>
</template>
