import { Mark, mergeAttributes } from "@tiptap/core";
import { InlineKind } from "@waypoints/domain";

/**
 * Minimal mention mark — registers the schema type so Tiptap can parse
 * content containing mention marks without crashing. Full mention editing
 * (autocomplete, insertion UI) is deferred to a later milestone.
 */
export const Mention = Mark.create({
  name: InlineKind.Mention,

  addAttributes() {
    return {
      id: { default: null },
    };
  },

  parseHTML() {
    return [{ tag: `span[data-inline="${InlineKind.Mention}"]` }];
  },

  renderHTML({ HTMLAttributes }) {
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-inline": InlineKind.Mention,
        class: "mention",
      }),
      0,
    ];
  },
});
