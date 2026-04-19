import { Mark, mergeAttributes, getMarkRange } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { InlineKind } from "@waypoints/domain";

/**
 * Mention mark — renders inline mention references. The segment text is
 * pre-hydrated by the backend to show the landmark title rather than a UUID.
 *
 * Mentions are atomic: the cursor cannot be placed inside them, and
 * backspace/delete removes the entire mention as a unit.
 *
 * Clicks on mentions while the editor is focused are suppressed to prioritize
 * editing; navigation is handled by the read-only InlineMention.vue component.
 */
export const Mention = Mark.create({
  name: InlineKind.Mention,

  inclusive: false,

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
    const markType = this.type;

    return [
      new Plugin({
        key: new PluginKey("mentionBehavior"),
        props: {
          handleClick(view, _pos, event) {
            const target = event.target as HTMLElement;
            if (target.closest?.('[data-inline="mention"]')) {
              return true;
            }
            return false;
          },

          handleKeyDown(view, event) {
            const { state } = view;
            const { selection } = state;
            const { $from, $to, empty } = selection;

            if (event.key === "Backspace" && empty) {
              // Check if the character immediately before the cursor is inside a mention
              if ($from.pos <= 1) return false;
              const before = $from.doc.resolve($from.pos - 1);
              const range = getMarkRange(before, markType);
              // Only delete if the mention ends exactly at the cursor position
              if (range && range.to === $from.pos) {
                view.dispatch(state.tr.delete(range.from, range.to));
                return true;
              }
            }

            if (event.key === "Delete" && empty) {
              // Check if the character immediately after the cursor is inside a mention
              if ($from.pos >= state.doc.content.size - 1) return false;
              const after = $from.doc.resolve($from.pos + 1);
              const range = getMarkRange(after, markType);
              // Only delete if the mention starts exactly at the cursor position
              if (range && range.from === $from.pos) {
                view.dispatch(state.tr.delete(range.from, range.to));
                return true;
              }
            }

            // If cursor is inside a mention (not at edges), prevent typing
            const mentionRange = getMarkRange($from, markType);
            if (mentionRange && !empty) {
              // Selection spans mention — let default behavior handle it
              return false;
            }
            if (
              mentionRange &&
              empty &&
              mentionRange.from < $from.pos &&
              $from.pos < mentionRange.to
            ) {
              // Cursor is inside a mention — skip to the nearest edge
              if (event.key === "ArrowLeft") {
                view.dispatch(
                  state.tr.setSelection(
                    state.selection.constructor.near(state.doc.resolve(mentionRange.from), -1),
                  ),
                );
                return true;
              }
              if (event.key === "ArrowRight") {
                view.dispatch(
                  state.tr.setSelection(
                    state.selection.constructor.near(state.doc.resolve(mentionRange.to)),
                  ),
                );
                return true;
              }
              // Block character input inside mention
              if (event.key.length === 1 && !event.metaKey && !event.ctrlKey) {
                return true;
              }
            }

            return false;
          },
        },
      }),
    ];
  },
});
