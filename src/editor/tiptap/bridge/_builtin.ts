import { markInputRule, markPasteRule, mergeAttributes } from "@tiptap/core";
import TipTapBold from "@tiptap/extension-bold";
import TipTapCode from "@tiptap/extension-code";
import TipTapItalic from "@tiptap/extension-italic";
import TipTapStrikethrough from "@tiptap/extension-strike";
import TipTapSubscript from "@tiptap/extension-subscript";
import TipTapSuperscript from "@tiptap/extension-superscript";
import TipTapUnderline from "@tiptap/extension-underline";
import { InlineKind } from "@waypoints/domain";

export const Bold = TipTapBold.extend({
  renderHTML({ HTMLAttributes }) {
    return [
      "strong",
      mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, {
        "data-inline": InlineKind.Bold,
      }),
      0,
    ];
  },
});

export const Code = TipTapCode.extend({
  renderHTML({ HTMLAttributes }) {
    return [
      "code",
      mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, {
        "data-inline": InlineKind.Code,
      }),
      0,
    ];
  },
});

export const Italic = TipTapItalic.extend({
  renderHTML({ HTMLAttributes }) {
    return [
      "em",
      mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, {
        "data-inline": InlineKind.Italic,
      }),
      0,
    ];
  },
});

export const Strikethrough = TipTapStrikethrough.extend({
  name: "strikethrough",
  renderHTML({ HTMLAttributes }) {
    return [
      "s",
      mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, {
        "data-inline": InlineKind.Strikethrough,
      }),
      0,
    ];
  },
});

const subscriptInputRE = /(?:^|\s)(~([^~]+)~)$/;
const subscriptPasteRE = /(?:^|\s)(~([^~]+)~)/g;

export const Subscript = TipTapSubscript.extend({
  renderHTML({ HTMLAttributes }) {
    return [
      "sub",
      mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, {
        "data-inline": InlineKind.Subscript,
      }),
      0,
    ];
  },

  addInputRules() {
    return [markInputRule({ find: subscriptInputRE, type: this.type })];
  },

  addPasteRules() {
    return [markPasteRule({ find: subscriptPasteRE, type: this.type })];
  },
});

const superscriptInputRE = /(?:^|\s)(\^([^^]+)\^)$/;
const superscriptPasteRE = /(?:^|\s)(\^([^^]+)\^)/g;

export const Superscript = TipTapSuperscript.extend({
  renderHTML({ HTMLAttributes }) {
    return [
      "sup",
      mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, {
        "data-inline": InlineKind.Superscript,
      }),
      0,
    ];
  },

  addInputRules() {
    return [markInputRule({ find: superscriptInputRE, type: this.type })];
  },

  addPasteRules() {
    return [markPasteRule({ find: superscriptPasteRE, type: this.type })];
  },
});

const underlineInputRE = /(?:^|\s)(\+\+([^+]+)\+\+)$/;
const underlinePasteRE = /(?:^|\s)(\+\+([^+]+)\+\+)/g;

export const Underline = TipTapUnderline.extend({
  renderHTML({ HTMLAttributes }) {
    return [
      "u",
      mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, {
        "data-inline": InlineKind.Underline,
      }),
      0,
    ];
  },

  addInputRules() {
    return [markInputRule({ find: underlineInputRE, type: this.type })];
  },

  addPasteRules() {
    return [markPasteRule({ find: underlinePasteRE, type: this.type })];
  },
});
