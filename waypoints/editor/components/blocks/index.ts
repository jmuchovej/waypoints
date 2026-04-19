import { ContentType, isLandmark, type MarkData } from "@waypoints/domain";
import { defineAsyncComponent, type Component } from "vue";

import LandmarkBlock from "./LandmarkBlock.vue";
import ParagraphBlock from "./ParagraphBlock.vue";
import HeadingBlock from "./HeadingBlock.vue";
import ListItemBlock from "./ListItemBlock.vue";
import DividerBlock from "./DividerBlock.vue";
import CodeBlock from "./CodeBlock.vue";
import MathBlock from "./MathBlock.vue";
import TableBlock from "./TableBlock.vue";

// Compound blocks import Block.vue (circular dependency) — use defineAsyncComponent
// to defer resolution, with suspensible: false to avoid the <Suspense> wrapper
// that causes DOM teardown on re-render.
const BlockquoteBlock = defineAsyncComponent({
  loader: () => import("./BlockquoteBlock.vue"),
  suspensible: false,
});
const CalloutBlock = defineAsyncComponent({
  loader: () => import("./CalloutBlock.vue"),
  suspensible: false,
});

export const registry = (data: MarkData): Component => {
  if (isLandmark(data)) {
    return LandmarkBlock;
  }

  switch (data.type) {
    case ContentType.Paragraph:
      return ParagraphBlock;
    case ContentType.Heading1:
    case ContentType.Heading2:
    case ContentType.Heading3:
      return HeadingBlock;
    case ContentType.ListItem:
    case ContentType.EnumItem:
      return ListItemBlock;
    case ContentType.Divider:
      return DividerBlock;
    case ContentType.Code:
      return CodeBlock;
    case ContentType.Math:
      return MathBlock;
    case ContentType.Table:
      return TableBlock;
    case ContentType.Blockquote:
      return BlockquoteBlock;
    case ContentType.Callout:
      return CalloutBlock;
  }
};

type AriaSemantics = {
  role: string;
  [key: string]: string;
};

export const ariaSemantics = (data: MarkData): AriaSemantics => {
  if (isLandmark(data)) return { role: "article" };

  switch (data.type) {
    case ContentType.Paragraph:
      return { role: "paragraph" };
    case ContentType.Heading1:
      return { role: "heading", "aria-level": "1" };
    case ContentType.Heading2:
      return { role: "heading", "aria-level": "2" };
    case ContentType.Heading3:
      return { role: "heading", "aria-level": "3" };
    case ContentType.ListItem:
    case ContentType.EnumItem:
      return { role: "listitem" };
    case ContentType.Divider:
      return { role: "separator" };
    case ContentType.Code:
      return { role: "code" };
    case ContentType.Math:
      return { role: "math" };
    case ContentType.Table:
      return { role: "table" };
    case ContentType.Blockquote:
      return { role: "blockquote" };
    case ContentType.Callout:
      return { role: "note" };
  }
};
