import { Mark, mergeAttributes } from "@tiptap/core";
import { InlineKind } from "@waypoints/domain";

/**
 * Highlight mark — applies a background color to selected text.
 * Stores a `color` attribute matching the domain `Color` enum (lowercase).
 * Renders with `data-theme` for CSS variable resolution.
 */
export const Highlight = Mark.create({
  name: InlineKind.Highlight,

  addAttributes() {
    return {
      color: { default: "yellow" },
    };
  },

  parseHTML() {
    return [{ tag: `span[data-inline="${InlineKind.Highlight}"]` }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-inline": InlineKind.Highlight,
        "data-theme": HTMLAttributes.color,
      }),
      0,
    ];
  },

  addKeyboardShortcuts() {
    return {
      "Mod-Shift-e": () => this.editor.commands.toggleMark(this.name, { color: "yellow" }),
    };
  },
});
