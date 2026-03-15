<script setup lang="ts">
import type { ContentNode } from "~~/editor";

defineProps<{ block: ContentNode<"divider"> }>();
</script>

<template>
  <div class="border-t border-default my-4" />
</template>
