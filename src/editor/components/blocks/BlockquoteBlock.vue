<script setup lang="ts">
import { ContentType } from "@waypoints/domain";
import type { ContentNode } from "~~/editor";
import { BlockOrchestratorKey, type OrchestratorContext } from "../../composables";
import Block from "../Block.vue";

const props = defineProps<{ block: ContentNode<"blockquote"> }>();

const orchestrator = inject<OrchestratorContext>(BlockOrchestratorKey);

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
  <div class="flex gap-x-2 gap-y-1">
    <div class="bg-accented w-1.5 rounded-xs" />
    <div class="italic flex-1">
      <Block v-for="child in childBlocks" :key="child.id" :block="child" />
      <p v-if="childBlocks.length === 0" class="text-dimmed text-sm italic">Empty quote</p>
    </div>
  </div>
</template>
