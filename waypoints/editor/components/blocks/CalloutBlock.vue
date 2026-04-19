<script setup lang="ts">
import type { ContentNode } from "~~/editor";
import type { OrchestratorContext } from "../../composables";
import Block from "../Block.vue";
import ipc from "~/ipc";

defineOptions({ inheritAttrs: false });

const props = defineProps<{ block: ContentNode<"callout"> }>();

const orchestrator = inject<OrchestratorContext>("block-orchestrator");

const CALLOUT_ICONS = [
  "💡",
  "ℹ️",
  "⚠️",
  "🚨",
  "✅",
  "❌",
  "📝",
  "📌",
  "🔥",
  "💎",
  "🎯",
  "🧠",
  "💬",
  "📣",
  "🔗",
  "⭐",
];

async function setIcon(icon: string) {
  if (!orchestrator) return;
  try {
    const result = await ipc.editor.updateMark(
      props.block.id,
      { ...props.block.data, icon },
      orchestrator.landmarkID.value,
    );
    orchestrator.blocks.value = result.blocks;
  } catch (e) {
    console.error("setIcon failed:", e);
  }
}

const isGroupActive = computed(() => {
  if (!orchestrator) return false;
  return orchestrator.focus.groupID.value === props.block.id;
});

provide("group-active", isGroupActive);
const compoundDepth = computed(() => props.block.depth);
provide("compound-parent-depth", compoundDepth);

/** All descendants of this compound block — rendered flat with depth-based indentation. */
const childBlocks = computed(() => {
  if (!orchestrator) return [];
  const allBlocks = orchestrator.blocks.value;
  const descendants = new Set<string>();

  for (const b of allBlocks) {
    if (b.parent_id === props.block.id) {
      descendants.add(b.id);
    } else if (descendants.has(b.parent_id ?? "")) {
      descendants.add(b.id);
    }
  }

  return allBlocks.filter((b) => descendants.has(b.id));
});
</script>

<template>
  <div class="wrapper" :data-theme="block.data.theme?.color ?? 'default'">
    <UPopover :content="{ align: 'start' }">
      <button class="prefix" type="button">
        {{ block.data.icon }}
      </button>
      <template #content>
        <div class="grid grid-cols-8 gap-0.5 p-2">
          <button
            v-for="icon in CALLOUT_ICONS"
            :key="icon"
            type="button"
            class="size-8 flex items-center justify-center rounded hover:bg-elevated cursor-pointer text-base"
            :class="{ 'bg-primary/10': icon === block.data.icon }"
            @click="setIcon(icon)"
          >
            {{ icon }}
          </button>
        </div>
      </template>
    </UPopover>
    <div class="body">
      <Block v-for="child in childBlocks" :key="child.id" :block="child" />
      <p v-if="childBlocks.length === 0" class="text-dimmed text-sm italic">Empty callout</p>
    </div>
  </div>
</template>
