import { InlineKind, type InlineMark } from "@waypoints/domain";
import type { Component } from "vue";

import InlineBold from "./InlineBold.vue";
import InlineItalic from "./InlineItalic.vue";
import InlineUnderline from "./InlineUnderline.vue";
import InlineStrikethrough from "./InlineStrikethrough.vue";
import InlineSuperscript from "./InlineSuperscript.vue";
import InlineSubscript from "./InlineSubscript.vue";
import InlineCode from "./InlineCode.vue";
import InlineHighlight from "./InlineHighlight.vue";
import InlineLink from "./InlineLink.vue";
import InlineMention from "./InlineMention.vue";
import InlineSpoiler from "./InlineSpoiler.vue";
import InlineColor from "./InlineColor.vue";
import InlineMath from "./InlineMath.vue";
import InlineComment from "./InlineComment.vue";

export const registry = (mark: InlineMark): Component | undefined => {
  switch (mark.kind) {
    case InlineKind.Bold:
      return InlineBold;
    case InlineKind.Italic:
      return InlineItalic;
    case InlineKind.Underline:
      return InlineUnderline;
    case InlineKind.Strikethrough:
      return InlineStrikethrough;
    case InlineKind.Superscript:
      return InlineSuperscript;
    case InlineKind.Subscript:
      return InlineSubscript;
    case InlineKind.Code:
      return InlineCode;
    case InlineKind.Link:
      return InlineLink;
    case InlineKind.Mention:
      return InlineMention;
    case InlineKind.Highlight:
      return InlineHighlight;
    case InlineKind.Color:
      return InlineColor;
    case InlineKind.Math:
      return InlineMath;
    case InlineKind.Spoiler:
      return InlineSpoiler;
    case InlineKind.Comment:
      return InlineComment;
  }
};
