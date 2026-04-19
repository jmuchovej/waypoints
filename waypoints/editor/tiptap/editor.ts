import type { JSONContent, EditorOptions as TipTapEditorOptions } from "@tiptap/core";
import { Editor } from "@tiptap/core";
import TipTapDocument from "@tiptap/extension-document";
import TipTapEmoji from "@tiptap/extension-emoji";
import TipTapLink from "@tiptap/extension-link";
import TipTapParagraph from "@tiptap/extension-paragraph";
import TipTapPlaceholder from "@tiptap/extension-placeholder";
import TipTapText from "@tiptap/extension-text";
import { UndoRedo } from "@tiptap/extensions";
import type { EditorProps } from "@tiptap/pm/view";
import type { RichText } from "@waypoints/domain";
import { CommandMenu, type CommandMenuDelegate } from "./extensions/command-menu";
import { MarkdownShortcuts, type BlockTypeDelegate } from "./extensions/markdown-shortcuts";
import {
  Bold,
  Code,
  Italic,
  Strikethrough,
  Subscript,
  Superscript,
  Underline,
} from "./bridge/_builtin";
import { Math } from "./bridge/math";
import { Mention } from "./bridge/mention";
import { Spoiler } from "./bridge/spoiler";
import { toTipTap } from "./converters";

export type { Editor } from "@tiptap/core";

const InlineDocument = TipTapDocument.extend({
  content: "paragraph",
});

export function mkDocument(rt: RichText): JSONContent {
  const content = toTipTap(rt);
  return {
    type: "doc",
    content: [{ type: "paragraph", content }],
  };
}

export interface EditorOptions {
  richText: RichText;
  blockID?: string;
  placeholder?: string;
  onUpdate: TipTapEditorOptions["onUpdate"];
  onKeyDown?: EditorProps["handleKeyDown"];
  menu?: CommandMenuDelegate;
  orchestrator?: BlockTypeDelegate;
}

export function createEditor(element: HTMLElement, options: EditorOptions): Editor {
  const commandMenu = CommandMenu.configure({
    blockID: options.blockID,
    menu: options.menu,
    triggers: {
      slash: /(?:^|\s)\/(\S*)$/,
      mention: /(?:^|\s)@(\S*)$/,
    },
  });
  const md = MarkdownShortcuts.configure({
    blockID: options.blockID,
    orchestrator: options.orchestrator,
  });

  const editor = new Editor({
    element,
    content: mkDocument(options.richText),
    extensions: [
      InlineDocument,
      TipTapText,
      TipTapParagraph,
      TipTapLink,
      Bold,
      Italic,
      Strikethrough,
      Code,
      Underline,
      Subscript,
      Superscript,
      Spoiler,
      Math,
      Mention,
      TipTapPlaceholder.configure({
        dataAttribute: "placeholder",
        placeholder: options.placeholder ?? "Type something…",
      }),
      TipTapEmoji,
      UndoRedo.configure({
        depth: 100,
        newGroupDelay: 500,
      }).extend({
        // Disable built-in Cmd+Z shortcuts — we route through native menu events
        addKeyboardShortcuts: () => ({}),
      }),
      commandMenu,
      md,
    ],
    injectCSS: false,
    editorProps: {
      attributes: {
        class: "inline-editor",
        spellcheck: "true",
      },
      handleKeyDown: options.onKeyDown,
    },
    onUpdate: options.onUpdate,
  });

  return editor;
}
