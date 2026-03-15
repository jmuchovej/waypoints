<script setup lang="ts">
import type { TextSegment } from "@waypoints/domain";
import InlineBold from "./inline/InlineBold.vue";
import InlineItalic from "./inline/InlineItalic.vue";
import InlineUnderline from "./inline/InlineUnderline.vue";
import InlineStrikethrough from "./inline/InlineStrikethrough.vue";
import InlineSuperscript from "./inline/InlineSuperscript.vue";
import InlineSubscript from "./inline/InlineSubscript.vue";
import InlineCode from "./inline/InlineCode.vue";
import InlineHighlight from "./inline/InlineHighlight.vue";
import InlineLink from "./inline/InlineLink.vue";
import InlineMention from "./inline/InlineMention.vue";
import InlineSpoiler from "./inline/InlineSpoiler.vue";
import InlineColor from "./inline/InlineColor.vue";
import InlineMath from "./inline/InlineMath.vue";
import InlineComment from "./inline/InlineComment.vue";

const props = defineProps<{
  segment: TextSegment;
}>();

const markComponentMap: Record<string, Component> = {
  bold: InlineBold,
  italic: InlineItalic,
  underline: InlineUnderline,
  strikethrough: InlineStrikethrough,
  superscript: InlineSuperscript,
  subscript: InlineSubscript,
  code: InlineCode,
  highlight: InlineHighlight,
  link: InlineLink,
  mention: InlineMention,
  spoiler: InlineSpoiler,
  color: InlineColor,
  math: InlineMath,
  comment: InlineComment,
};

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
    const Comp = markComponentMap[mark.kind];
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
