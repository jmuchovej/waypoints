<script setup lang="ts">
import type { ContentNode } from "~~/editor";
import RichTextEditor from "../RichTextEditor.vue";

const props = defineProps<{ block: ContentNode<"paragraph"> }>();
</script>

<template>
  <RichTextEditor
    :block-id="block.id"
    :rich-text="block.data.rich_text"
    placeholder="Type a 'space' for AI or '/' for commands.."
  />
</template>
