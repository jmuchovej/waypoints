<script lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

interface TextReadingProps {
  value: Reading | undefined;
  feature: Feature;
}

interface TextReadingEmits {
  update: [reading: Reading];
}
</script>
<script setup lang="ts">
import type { RichText } from "@waypoints/domain";
import RichTextEditor from "../RichTextEditor.vue";

const props = defineProps<TextReadingProps>();
const emit = defineEmits<TextReadingEmits>();

const richText = computed<RichText>(() => {
  if (props.value?.type === "text") return props.value.value;
  return { segments: [], plain_text: "" };
});

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function onUpdate(rt: RichText) {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    emit("update", { type: "text", value: rt });
  }, 250);
}
</script>

<template>
  <RichTextEditor
    :rich-text="richText"
    :on-update="onUpdate"
    placeholder="Empty"
    class="reading-text"
  />
</template>

<style scoped>
@reference "~/assets/css/main.css";

.reading-text {
  @apply w-full rounded-md text-sm/4;
  @apply px-2.5 py-2;
  @apply text-highlighted bg-transparent;
  @apply hover:bg-elevated focus-within:bg-elevated;
}
</style>
