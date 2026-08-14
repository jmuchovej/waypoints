<script setup lang="ts">
  import type { LandmarkNode } from "#imports";

  const props = defineProps<{ block: LandmarkNode }>();

  const title = computed(() => {
    const d = props.block.data;
    return d.title.plain_text ?? (d.title.segments.map((s) => s.text).join("") || null);
  });
</script>

<template>
  <div class="border-default text-dimmed my-1 rounded-lg border p-3 text-sm">
    🔗 {{ title ?? block.id }}
  </div>
</template>
