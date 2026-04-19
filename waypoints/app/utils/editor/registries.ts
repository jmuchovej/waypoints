import {
  InlineKind,
  type InlineMark,
  type Feature,
  ContentType,
  isLandmark,
  type MarkData,
} from "@waypoints/domain";
import { defineAsyncComponent, type Component } from "vue";

import {
  InlineBold,
  InlineItalic,
  InlineUnderline,
  InlineStrikethrough,
  InlineSuperscript,
  InlineSubscript,
  InlineCode,
  InlineHighlight,
  InlineLink,
  InlineMention,
  InlineSpoiler,
  InlineColor,
  InlineMath,
  InlineComment,
} from "#components";

export const inlineComponent = (mark: InlineMark): Component | undefined => {
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

import {
  ReadingText,
  ReadingNumber,
  ReadingCheckbox,
  ReadingEmail,
  ReadingURL,
  ReadingPhone,
  ReadingDate,
  ReadingReference,
  ReadingFormula,
  ReadingSelect,
} from "#components";

export const featureComponent = (feature: Feature): Component => {
  switch (feature.kind.type) {
    case "text":
      return ReadingText;
    case "number":
      return ReadingNumber;
    case "checkbox":
      return ReadingCheckbox;
    case "email":
      return ReadingEmail;
    case "url":
      return ReadingURL;
    case "phone":
      return ReadingPhone;
    case "date":
      return ReadingDate;
    case "reference":
      return ReadingReference;
    case "formula":
      return ReadingFormula;
    case "select":
    case "multi-select":
    case "grouped-select":
    case "grouped-multi-select":
      return ReadingSelect;
  }
};

import {
  BlockLandmark,
  BlockParagraph,
  BlockHeading,
  BlockListItem,
  BlockDivider,
  BlockCode,
  BlockMath,
  BlockTable,
} from "#components";

// Compound blocks import Block.vue (circular dependency) — use defineAsyncComponent
// to defer resolution, with suspensible: false to avoid the <Suspense> wrapper
// that causes DOM teardown on re-render. Nuxt's Lazy* components omit
// suspensible: false, so we build these two ourselves.
const BlockBlockquote = defineAsyncComponent({
  loader: () => import("~/components/block/Blockquote.vue"),
  suspensible: false,
});
const BlockCallout = defineAsyncComponent({
  loader: () => import("~/components/block/Callout.vue"),
  suspensible: false,
});

export const blockComponent = (data: MarkData): Component => {
  if (isLandmark(data)) {
    return BlockLandmark;
  }

  switch (data.type) {
    case ContentType.Paragraph:
      return BlockParagraph;
    case ContentType.Heading1:
    case ContentType.Heading2:
    case ContentType.Heading3:
      return BlockHeading;
    case ContentType.ListItem:
    case ContentType.EnumItem:
      return BlockListItem;
    case ContentType.Divider:
      return BlockDivider;
    case ContentType.Code:
      return BlockCode;
    case ContentType.Math:
      return BlockMath;
    case ContentType.Table:
      return BlockTable;
    case ContentType.Blockquote:
      return BlockBlockquote;
    case ContentType.Callout:
      return BlockCallout;
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
