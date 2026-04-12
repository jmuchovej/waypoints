<script setup lang="ts">
import { ContentType } from "@waypoints/domain";
import type { ContentNode } from "~~/editor";

const props = defineProps<{ block: ContentNode<"code"> }>();

const lines = computed(() => props.block.data.lines.join("\n"));
</script>

<template>
  <pre class="text-sm leading-6 border border-muted bg-muted rounded-md px-4 py-3">
    <code>{{ lines }}</code>
  </pre>
</template>
