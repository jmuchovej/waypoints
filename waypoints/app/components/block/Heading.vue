<script setup lang="ts">
  import { RichTextEditor } from "#components";
  import type { ContentNode } from "#imports";

  const props = defineProps<{
    block: ContentNode<"heading-1" | "heading-2" | "heading-3">;
  }>();
</script>

<template>
  <RichTextEditor :block-id="block.id" :rich-text="block.data.rich_text" placeholder="Heading" />
</template>
