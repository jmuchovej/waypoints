import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

export type CommandMenuKind = "slash" | "mention" | "emoji";

export interface CommandMenuDelegate {
  show(kind: CommandMenuKind, blockID: string, query: string, pos: { x: number; y: number }): void;
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
                menu.show(state.kind!, blockID, state.query, {
                  x: coords.left,
                  y: coords.bottom + 4,
                });
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
