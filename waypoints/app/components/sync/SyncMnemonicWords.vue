<script setup lang="ts">
const props = defineProps<{
  phrase: string;
}>();

const words = computed(() => props.phrase.trim().split(/\s+/));
</script>

<template>
  <ol class="grid grid-cols-2 sm:grid-cols-3 gap-2 w-full">
    <li
      v-for="(word, i) in words"
      :key="i"
      class="flex items-baseline gap-2 rounded-md bg-elevated px-3 py-1.5 text-sm"
    >
      <span class="text-dimmed tabular-nums text-xs w-4 shrink-0 text-right">{{ i + 1 }}</span>
      <span class="font-medium text-highlighted">{{ word }}</span>
    </li>
  </ol>
</template>
