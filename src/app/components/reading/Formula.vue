<script lang="ts">
  import type { Reading } from "@waypoints/domain";

  export interface FormulaReadingProps {
    value: Reading | undefined;
  }
</script>
<script setup lang="ts">
  const props = defineProps<FormulaReadingProps>();

  const formula = computed(() => {
    if (props.value?.type === "formula") return props.value.value;
    return null;
  });
</script>

<template>
  <span class="text-dimmed font-mono text-sm">
    {{ formula ?? "—" }}
  </span>
</template>
