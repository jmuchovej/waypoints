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
  <span class="prefix">
    <template v-if="isContent(block.data, ContentType.EnumItem)"> {{ enumIndex ?? 1 }}. </template>
    <template v-else>&bull;</template>
  </span>
  <RichTextEditor :block-id="block.id" :rich-text="block.data.rich_text" placeholder="Empty" />
</template>
