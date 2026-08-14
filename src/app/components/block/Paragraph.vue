<script setup lang="ts">
  import { RichTextEditor } from "#components";
  import type { ContentNode } from "#imports";

  const props = defineProps<{ block: ContentNode<"paragraph"> }>();
</script>

<template>
  <RichTextEditor
    :block-id="block.id"
    :rich-text="block.data.rich_text"
    placeholder="Type a 'space' for AI or '/' for commands." />
</template>
