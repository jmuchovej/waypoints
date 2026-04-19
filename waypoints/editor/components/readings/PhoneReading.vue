<script lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

export interface PhoneReadingProps {
  value: Reading | undefined;
  feature: Feature;
}

export interface PhoneReadingEmits {
  update: [reading: Reading];
}
</script>

<script setup lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

// TODO: expand country-code support beyond US (+1).
// Currently the country code is hardcoded; the second element of Reading::Phone
// is the user-entered number string. A future iteration should add a country
// picker (probably USelectMenu with flags) and persist both fields properly.
const COUNTRY_CODE = "+1";

const props = defineProps<PhoneReadingProps>();
const emit = defineEmits<PhoneReadingEmits>();

const localValue = ref<string>(props.value?.type === "phone" ? props.value.value[1] : "");

watch(
  () => props.value,
  (val) => {
    localValue.value = val?.type === "phone" ? val.value[1] : "";
  },
);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function onInput(val: string) {
  localValue.value = val;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    emit("update", { type: "phone", value: [COUNTRY_CODE, val] });
  }, 250);
}
</script>

<template>
  <UInput
    :model-value="localValue"
    type="tel"
    placeholder="Empty"
    size="md"
    variant="ghost"
    @update:model-value="onInput"
  />
</template>
