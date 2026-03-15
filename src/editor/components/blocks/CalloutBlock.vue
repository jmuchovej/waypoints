<script setup lang="ts">
import type { ContentNode } from "~~/editor";
import type { OrchestratorContext } from "../../composables";
import Block from "../Block.vue";

const props = defineProps<{ block: ContentNode<"callout"> }>();

const orchestrator = inject<OrchestratorContext>("block-orchestrator");

const isGroupActive = computed(() => {
  if (!orchestrator) return false;
  return orchestrator.focus.groupID.value === props.block.id;
});

provide("group-active", isGroupActive);
provide("compound-parent-depth", props.block.depth);

const childBlocks = computed(() => {
  if (!orchestrator) return [];
  return orchestrator.blocks.value.filter((b) => b.parent_id === props.block.id);
});
</script>

<template>
  <div :data-color="block.data.background_color" class="flex gap-x-4 border rounded-lg p-3">
    <span class="text-xl shrink-0">{{ block.data.icon }}</span>
    <div class="flex-1">
      <Block v-for="child in childBlocks" :key="child.id" :block="child" />
      <p v-if="childBlocks.length === 0" class="text-dimmed text-sm italic">Empty callout</p>
    </div>
  </div>
</template>
