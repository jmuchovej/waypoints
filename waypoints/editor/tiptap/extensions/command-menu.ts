import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

export type CommandMenuKind = "slash" | "mention" | "emoji";

export type OnSelectCallback = (value: unknown) => void;

export interface CommandMenuDelegate {
  show(
    kind: CommandMenuKind,
    blockID: string,
    query: string,
    pos: { x: number; y: number },
    onSelect?: OnSelectCallback,
  ): void;
  updateQuery(query: string): void;
  hide(): void;
}

export interface CommandMenuOptions {
  blockID?: string;
  menu?: CommandMenuDelegate;
  triggers: Partial<Record<CommandMenuKind, RegExp>>;
}

interface MenuState {
  active: boolean;
  kind: CommandMenuKind | null;
  query: string;
}

const inactive: MenuState = { active: false, kind: null, query: "" };

const name = "command-menu";
const key = new PluginKey(name);

/**
 * Unified command-menu trigger extension.
 *
 * Detects configurable trigger patterns (e.g. `/` for slash, `@` for mention)
 * and drives the shared CommandMenuContext overlay.
 */
export const CommandMenu = Extension.create<CommandMenuOptions>({
  name,

  addOptions() {
    return {
      triggers: {},
    };
  },

  addProseMirrorPlugins() {
    const { blockID, triggers, menu } = this.options;

    const entries = Object.entries(triggers) as [CommandMenuKind, RegExp][];
    if (!entries.length || !menu || !blockID) return [];

    return [
      new Plugin({
        key,
        state: {
          init(): MenuState {
            return inactive;
          },
          apply(tr): MenuState {
            const { selection } = tr;
            if (!selection.empty) return inactive;

            const $pos = selection.$from;
            const textBefore = $pos.parent.textBetween(0, $pos.parentOffset, undefined, "\ufffc");

            for (const [kind, pattern] of entries) {
              const match = textBefore.match(pattern);
              if (match) {
                return { active: true, kind, query: match[1] ?? "" };
              }
            }

            return inactive;
          },
        },
        view() {
          let wasActive = false;

          return {
            update(view) {
              const state = key.getState(view.state) as MenuState | undefined;
              if (!state) return;

              if (state.active && !wasActive) {
                const coords = view.coordsAtPos(view.state.selection.from);

                // Build an onSelect callback that replaces the trigger text
                // with the selected content (e.g., mention-marked text).
                const onSelect: OnSelectCallback = (value) => {
                  const currentState = key.getState(view.state) as MenuState | undefined;
                  if (!currentState?.active) return;

                  // Find the trigger range: from the trigger character to the cursor
                  const { selection } = view.state;
                  const $pos = selection.$from;
                  const textBefore = $pos.parent.textBetween(
                    0,
                    $pos.parentOffset,
                    undefined,
                    "\ufffc",
                  );

                  // Find the current trigger's match to determine replacement range
                  const kind = currentState.kind!;
                  const pattern = triggers[kind];
                  if (!pattern) return;

                  const match = textBefore.match(pattern);
                  if (!match) return;

                  // The match starts at this offset within the parent node
                  const matchStart = $pos.start() + match.index!;
                  // Skip leading whitespace if the match includes it
                  const triggerStart = matchStart + (match[0].length - match[0].trimStart().length);
                  const triggerEnd = $pos.pos;

                  // Dispatch the replacement — the menu component provides the
                  // replacement content as a ProseMirror insertable value.
                  if (typeof value === "object" && value !== null && "type" in value) {
                    // ProseMirror-compatible content node (e.g., text with marks)
                    const tr = view.state.tr.replaceWith(
                      triggerStart,
                      triggerEnd,
                      view.state.schema.nodeFromJSON(value),
                    );
                    view.dispatch(tr);
                  }
                };

                menu.show(state.kind!, blockID, state.query, {
                  x: coords.left,
                  y: coords.bottom + 4,
                }, onSelect);
                wasActive = true;
              } else if (state.active && wasActive) {
                menu.updateQuery(state.query);
              } else if (!state.active && wasActive) {
                menu.hide();
                wasActive = false;
              }
            },
          };
        },
      }),
    ];
  },
});
