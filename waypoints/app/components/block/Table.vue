<script setup lang="ts">
import type { ContentNode } from "#imports";

defineProps<{ block: ContentNode<"table"> }>();
</script>

<template>
  <div class="border rounded p-2 text-sm text-dimmed">Table (stub)</div>
</template>
