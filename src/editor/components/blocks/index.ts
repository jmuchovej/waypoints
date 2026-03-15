import { ContentType, isLandmark, type MarkData } from "@waypoints/domain";
import { defineAsyncComponent, type Component } from "vue";

const LandmarkBlock = defineAsyncComponent(() => import("./LandmarkBlock.vue"));
const ParagraphBlock = defineAsyncComponent(() => import("./ParagraphBlock.vue"));
const HeadingBlock = defineAsyncComponent(() => import("./HeadingBlock.vue"));
const ListItemBlock = defineAsyncComponent(() => import("./ListItemBlock.vue"));
const DividerBlock = defineAsyncComponent(() => import("./DividerBlock.vue"));
const CodeBlock = defineAsyncComponent(() => import("./CodeBlock.vue"));
const MathBlock = defineAsyncComponent(() => import("./MathBlock.vue"));
const TableBlock = defineAsyncComponent(() => import("./TableBlock.vue"));
const BlockquoteBlock = defineAsyncComponent(() => import("./BlockquoteBlock.vue"));
const CalloutBlock = defineAsyncComponent(() => import("./CalloutBlock.vue"));

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
