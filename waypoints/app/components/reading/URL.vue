<script lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

export interface URLReadingProps {
  value: Reading | undefined;
  feature: Feature;
}

export interface URLReadingEmits {
  update: [reading: Reading];
}
</script>

<script setup lang="ts">
const props = defineProps<URLReadingProps>();
const emit = defineEmits<URLReadingEmits>();

const localValue = ref<string>(props.value?.type === "url" ? props.value.value : "");

watch(
  () => props.value,
  (val) => {
    localValue.value = val?.type === "url" ? val.value : "";
  },
);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function onInput(val: string) {
  localValue.value = val;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    emit("update", { type: "url", value: val });
  }, 250);
}
</script>

<template>
  <UInput
    :model-value="localValue"
    type="url"
    placeholder="Empty"
    size="md"
    @update:model-value="onInput"
  />
</template>
