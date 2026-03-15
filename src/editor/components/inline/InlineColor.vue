<script setup lang="ts">
const props = defineProps<{
  data: unknown;
}>();

const color = (props.data as Record<string, unknown> | null)?.color as string | undefined;
</script>

<template>
  <span data-inline="color" :style="{ color }"><slot /></span>
</template>
