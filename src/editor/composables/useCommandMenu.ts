import { readonly, ref } from "vue";
import type { CommandMenuKind } from "../tiptap/extensions/command-menu";

export type { CommandMenuKind };

export type MenuPosition = {
  x: number;
  y: number;
};

export interface CommandMenuState {
  kind: CommandMenuKind;
  query: string;
  position: MenuPosition;
  blockID: string;
  onSelect?: (value: unknown) => void;
}

export const CommandMenuKey = "command-menu";

export function useCommandMenu() {
  const activeMenu = ref<CommandMenuState | null>(null);

  function show(kind: CommandMenuKind, blockID: string, query: string, position: MenuPosition) {
    activeMenu.value = { kind, blockID, query, position };
  }

  function updateQuery(query: string) {
    if (activeMenu.value) {
      activeMenu.value = { ...activeMenu.value, query };
    }
  }

  function updatePosition(position: MenuPosition) {
    if (activeMenu.value) {
      activeMenu.value = { ...activeMenu.value, position };
    }
  }

  function hide() {
    activeMenu.value = null;
  }

  /** Returns true if the overlay consumed the key event. */
  function handleKey(event: KeyboardEvent): boolean {
    if (!activeMenu.value) return false;
    return ["ArrowUp", "ArrowDown", "Enter", "Escape"].includes(event.key);
  }

  const isOpen = computed(() => activeMenu.value !== null);
  const menuKind = computed(() => activeMenu.value?.kind ?? null);

  return {
    activeMenu: readonly(activeMenu),
    isOpen,
    menuKind,
    show,
    updateQuery,
    updatePosition,
    hide,
    handleKey,
  };
}

export type CommandMenuContext = ReturnType<typeof useCommandMenu>;
