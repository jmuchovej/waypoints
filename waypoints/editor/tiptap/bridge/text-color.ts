import { Mark, mergeAttributes } from "@tiptap/core";
import { InlineKind } from "@waypoints/domain";

/**
 * Text color mark — applies a foreground color to selected text.
 * Stores a `color` attribute matching the domain `Color` enum (lowercase).
 * Renders with `data-theme` for CSS variable resolution.
 */
export const TextColor = Mark.create({
  name: InlineKind.Color,

  addAttributes() {
    return {
      color: { default: "blue" },
    };
  },

  parseHTML() {
    return [{ tag: `span[data-inline="${InlineKind.Color}"]` }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-inline": InlineKind.Color,
        "data-theme": HTMLAttributes.color,
      }),
      0,
    ];
  },
});
