<script lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

export interface NumberReadingProps {
  value: Reading | undefined;
  feature: Feature;
}

export interface NumberReadingEmits {
  update: [reading: Reading];
}
</script>
<script setup lang="ts">
const props = defineProps<NumberReadingProps>();
const emit = defineEmits<NumberReadingEmits>();

const numberKind = computed(() => {
  if (props.feature.kind.type === "number") return props.feature.kind;
  return null;
});

const localValue = ref<number | undefined>(
  props.value?.type === "number" ? props.value.value : undefined,
);

watch(
  () => props.value,
  (val) => {
    localValue.value = val?.type === "number" ? val.value : undefined;
  },
);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function onChange(val: number) {
  localValue.value = val;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    emit("update", { type: "number", value: val });
  }, 250);
}
</script>

<template>
  <UInputNumber
    :model-value="localValue"
    :min="numberKind?.min ?? undefined"
    :max="numberKind?.max ?? undefined"
    :step="numberKind?.step ?? undefined"
    size="md"
    placeholder="Empty"
    @update:model-value="onChange"
  />
</template>
