import { Mark, mergeAttributes } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { InlineKind } from "@waypoints/domain";

/**
 * Mention mark — renders inline mention references. The segment text is
 * pre-hydrated by the backend to show the landmark title rather than a UUID.
 *
 * Clicks on mentions while the editor is focused are suppressed to prioritize
 * editing; navigation is handled by the read-only InlineMention.vue component.
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

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("mentionClick"),
        props: {
          handleClick(view, _pos, event) {
            const target = event.target as HTMLElement;
            if (target.closest?.('[data-inline="mention"]')) {
              // Suppress navigation while the editor is focused — prioritize editing.
              return true;
            }
            return false;
          },
        },
      }),
    ];
  },
});
