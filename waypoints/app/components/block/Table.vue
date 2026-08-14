<script setup lang="ts">
  import type { ContentNode } from "#imports";

  defineProps<{ block: ContentNode<"table"> }>();
</script>

<template>
  <div class="text-dimmed rounded border p-2 text-sm">Table (stub)</div>
</template>
