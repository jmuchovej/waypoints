<script setup lang="ts">
import type { LandmarkNode } from "~~/editor";

const props = defineProps<{ block: LandmarkNode }>();

const title = computed(() => {
  const d = props.block.data;
  return d.title.plain_text ?? (d.title.segments.map((s) => s.text).join("") || null);
});
</script>

<template>
  <div class="border border-default rounded-lg p-3 my-1 text-sm text-dimmed">
    🔗 {{ title ?? block.id }}
  </div>
</template>
