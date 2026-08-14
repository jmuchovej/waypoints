<script setup lang="ts">
  const props = defineProps<{
    phrase: string;
  }>();

  const words = computed(() => props.phrase.trim().split(/\s+/));
</script>

<template>
  <ol class="grid w-full grid-cols-2 gap-2 sm:grid-cols-3">
    <li
      v-for="(word, i) in words"
      :key="i"
      class="bg-elevated flex items-baseline gap-2 rounded-md px-3 py-1.5 text-sm">
      <span class="text-dimmed w-4 shrink-0 text-right text-xs tabular-nums">{{ i + 1 }}</span>
      <span class="text-highlighted font-medium">{{ word }}</span>
    </li>
  </ol>
</template>
