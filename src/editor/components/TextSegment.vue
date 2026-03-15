<script lang="ts">
import type { TextSegment } from "@waypoints/domain";

export interface TextSegmentProps {
  segment: TextSegment;
}
</script>
<script setup lang="ts">
import { registry } from "./inline";

const props = defineProps<TextSegmentProps>();

/**
 * Build a nested VNode tree: outermost mark wraps the next, etc.
 * Final leaf is a text node.
 */
function renderSegment() {
  const marks = props.segment.marks ?? [];
  if (marks.length === 0) {
    return props.segment.text;
  }

  // Build from innermost to outermost
  let inner: any = props.segment.text;
  for (const mark of [...marks].reverse()) {
    const Comp = registry(mark);
    if (Comp) {
      const captured = inner;
      inner = h(Comp, { data: mark.data }, () => captured);
    }
  }

  return inner;
}
</script>

<template>
  <component :is="renderSegment" />
</template>
