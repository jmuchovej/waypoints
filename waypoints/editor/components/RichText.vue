<script lang="ts">
import type { RichText } from "@waypoints/domain";

export type ReadonlyRichTextProps = {
  richText: RichText;
  placeholder?: string;
};
</script>

<script setup lang="ts">
import TextSegment from "./TextSegment.vue";

const props = defineProps<ReadonlyRichTextProps>();

const isEmpty = computed(() => {
  const rt = props.richText;
  if (!rt.segments.length) return true;
  return rt.segments.every((s) => !s.text);
});
</script>

<template>
  <span>
    <template v-if="isEmpty && placeholder">
      <span class="text-dimmed">{{ placeholder }}</span>
    </template>
    <template v-else-if="richText.segments.every((s) => !s.marks?.length)">
      {{ richText.plain_text || richText.segments.map((s) => s.text).join("") }}
    </template>
    <template v-else>
      <TextSegment v-for="(seg, i) in richText.segments" :key="i" :segment="seg" />
    </template>
  </span>
</template>
