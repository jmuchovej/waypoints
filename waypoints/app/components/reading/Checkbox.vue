<script lang="ts">
  import type { Reading, Feature } from "@waypoints/domain";

  export interface CheckboxReadingProps {
    value: Reading | undefined;
    feature: Feature;
  }

  export interface CheckboxReadingEmits {
    update: [reading: Reading];
  }
</script>

<script setup lang="ts">
  const props = defineProps<CheckboxReadingProps>();
  const emit = defineEmits<CheckboxReadingEmits>();

  const checked = computed(() => props.value?.type === "bool" && props.value.value);

  function onChange(val: boolean | "indeterminate") {
    emit("update", { type: "bool", value: val === true });
  }
</script>

<template>
  <UCheckbox :model-value="checked" @update:model-value="onChange" />
</template>
