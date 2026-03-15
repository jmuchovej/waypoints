<script setup lang="ts">
import { ContentType, isContent } from "@waypoints/domain";
import type { ContentNode } from "~~/editor";
import RichTextEditor from "../RichTextEditor.vue";

const props = defineProps<{
  block: ContentNode<"list-item" | "enum-item">;
  enumIndex?: number;
}>();
</script>

<template>
  <div class="relative py-0.5 leading-snug flex">
    <span class="select-none text-dimmed min-w-[2ch] pe-1 text-end">
      <template v-if="isContent(block.data, ContentType.EnumItem)">{{ enumIndex ?? 1 }}.</template>
      <template v-else>&bull;</template>
    </span>
    <RichTextEditor
      :block-id="block.id"
      :rich-text="block.data.rich_text"
      class="ps-2 flex-1"
      placeholder="List item"
    />
  </div>
</template>
