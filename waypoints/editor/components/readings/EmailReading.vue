<script lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

export interface EmailReadingProps {
  value: Reading | undefined;
  feature: Feature;
}

export interface EmailReadingEmits {
  update: [reading: Reading];
}
</script>
<script setup lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

const props = defineProps<EmailReadingProps>();
const emit = defineEmits<EmailReadingEmits>();

const localValue = ref<string>(props.value?.type === "email" ? props.value.value : "");

watch(
  () => props.value,
  (val) => {
    localValue.value = val?.type === "email" ? val.value : "";
  },
);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function onInput(val: string) {
  localValue.value = val;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    emit("update", { type: "email", value: val });
  }, 250);
}
</script>

<template>
  <UInput
    :model-value="localValue"
    type="email"
    placeholder="name@example.com"
    icon="i-lucide-mail"
    size="sm"
    @update:model-value="onInput"
  />
</template>
