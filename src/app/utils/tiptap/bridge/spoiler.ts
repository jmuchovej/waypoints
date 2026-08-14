import { getMarkRange, Mark, markInputRule, markPasteRule, mergeAttributes } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import { InlineKind } from "@waypoints/domain";

const inputRegex = /(?:^|\s)(\|\|([^|]+)\|\|)$/;
const pasteRegex = /(?:^|\s)(\|\|([^|]+)\|\|)/g;

const key = new PluginKey(InlineKind.Spoiler);

export const Spoiler = Mark.create({
  name: InlineKind.Spoiler,

  parseHTML() {
    return [{ tag: `span[data-inline="${InlineKind.Spoiler}"]` }];
  },

  renderHTML({ HTMLAttributes }) {
    return ["span", mergeAttributes(HTMLAttributes, { "data-inline": InlineKind.Spoiler }), 0];
  },

  addKeyboardShortcuts() {
    return {
      "Mod-Shift-h": () => this.editor.commands.toggleMark(this.name),
    };
  },

  addInputRules() {
    return [markInputRule({ find: inputRegex, type: this.type })];
  },

  addPasteRules() {
    return [markPasteRule({ find: pasteRegex, type: this.type })];
  },

  addProseMirrorPlugins() {
    const markType = this.type;

    return [
      new Plugin({
        key,
        state: {
          init() {
            return DecorationSet.empty;
          },

          apply(_tr, _old, _oldState, newState) {
            const { $from } = newState.selection;
            const range = getMarkRange($from, markType);

            if (!range) return DecorationSet.empty;

            return DecorationSet.create(newState.doc, [
              Decoration.inline(range.from, range.to, {
                class: "spoiler-active",
              }),
            ]);
          },
        },

        props: {
          decorations(state) {
            return key.getState(state);
          },
        },
      }),
    ];
  },
});
