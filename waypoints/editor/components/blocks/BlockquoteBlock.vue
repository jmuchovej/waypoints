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
const compoundDepth = computed(() => props.block.depth);
provide("compound-parent-depth", compoundDepth);

/** All descendants of this compound block — rendered flat with depth-based indentation,
 *  matching the same pattern DocumentBody uses at the root level. */
const childBlocks = computed(() => {
  if (!orchestrator) return [];
  const allBlocks = orchestrator.blocks.value;
  const descendants = new Set<string>();

  // Collect all descendants by propagating through parent chains
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
  <div class="flex gap-x-2 gap-y-1">
    <div class="bg-accented w-1.5 rounded-xs" />
    <div class="italic flex-1">
      <Block v-for="child in childBlocks" :key="child.id" :block="child" />
      <p v-if="childBlocks.length === 0" class="text-dimmed text-sm italic">Empty quote</p>
    </div>
  </div>
</template>
