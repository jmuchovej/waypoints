import { type Ref, ref, nextTick, computed, readonly } from "vue";
import { isCompoundType, type MarkID } from "@waypoints/domain";
import type { RenderNode } from "@waypoints/cartograph";

export type CursorPosition = "start" | "end";

export interface EditorHandle {
  type: string;
  focus(position?: CursorPosition): void;
  blur(): void;
  onCoords(x: number, y: number): void;
  onCoordsRange(x1: number, y1: number, x2: number, y2: number): void;
  getCursorOffset(): number;
  isCursorAtStart(): boolean;
  isCursorAtEnd(): boolean;
  isCursorOnFirstLine(): boolean;
  isCursorOnLastLine(): boolean;
  getContentLength(): number;
  flush(): Promise<void>;
  canUndo(): boolean;
  canRedo(): boolean;
  undo(): void;
  redo(): void;
}

type PendingFocus =
  | { type: "position"; position: CursorPosition }
  | { type: "coords"; x: number; y: number }
  | { type: "range"; x1: number; y1: number; x2: number; y2: number };

export function useBlockFocus(blocks?: Ref<RenderNode[]>) {
  const blockID = ref<MarkID | null>(null);
  const handles = new Map<MarkID, EditorHandle>();
  let pendingFocus: { id: MarkID; intent: PendingFocus } | null = null;

  /** Walk the parent_id chain to find the nearest compound ancestor (blockquote/callout/table). */
  const groupID = computed<MarkID | null>(() => {
    if (!blockID.value || !blocks?.value) return null;
    let current = blocks.value.find((b) => b.id === blockID.value);
    while (current) {
      const parent = blocks.value.find((b) => b.id === current!.parent_id);
      if (parent && isCompoundType(parent.data)) {
        return parent.id;
      }
      current = parent;
    }
    return null;
  });

  function registerEditor(id: MarkID, handle: EditorHandle) {
    handles.set(id, handle);
    // Execute any pending focus intent that was waiting for this handle.
    if (pendingFocus && pendingFocus.id === id) {
      const intent = pendingFocus.intent;
      pendingFocus = null;
      nextTick(() => {
        if (intent.type === "range") {
          handle.onCoordsRange(intent.x1, intent.y1, intent.x2, intent.y2);
        } else if (intent.type === "coords") {
          handle.onCoords(intent.x, intent.y);
        } else {
          handle.focus(intent.position);
        }
      });
    }
  }

  function unregisterEditor(id: MarkID) {
    handles.delete(id);
    if (blockID.value === id) {
      blockID.value = null;
    }
  }

  function getHandle(id: MarkID): EditorHandle | undefined {
    return handles.get(id);
  }

  function focusBlock(id: MarkID, position?: CursorPosition) {
    if (blockID.value && blockID.value !== id) {
      const prev = handles.get(blockID.value);
      prev?.blur();
    }
    blockID.value = id;
    const pos = position ?? "start";
    const handle = handles.get(id);
    if (handle) {
      // Handle already registered (e.g. navigating between live editors)
      nextTick(() => handle.focus(pos));
    } else {
      // Editor not mounted yet — registerEditor will pick this up
      pendingFocus = { id, intent: { type: "position", position: pos } };
    }
  }

  function focusBlockAt(id: MarkID, event: MouseEvent) {
    if (blockID.value && blockID.value !== id) {
      const prev = handles.get(blockID.value);
      prev?.blur();
    }
    blockID.value = id;
    const handle = handles.get(id);
    if (handle) {
      nextTick(() => handle.onCoords(event.clientX, event.clientY));
    } else {
      pendingFocus = { id, intent: { type: "coords", x: event.clientX, y: event.clientY } };
    }
  }

  /**
   * Focus a block and apply a text selection range from two screen coordinates.
   * Used for drag-to-select on non-focused blocks: the gesture is captured on the
   * static text, then applied after the editor mounts.
   */
  function focusBlockWithRange(id: MarkID, x1: number, y1: number, x2: number, y2: number) {
    if (blockID.value && blockID.value !== id) {
      const prev = handles.get(blockID.value);
      prev?.blur();
    }
    blockID.value = id;
    pendingFocus = { id, intent: { type: "range", x1, y1, x2, y2 } };
  }

  /**
   * Always stores a pending focus intent, even if a handle is already registered.
   * Used for structural mutations (block type changes) where the current editor
   * will be destroyed and a new one mounted.
   */
  function focusBlockDeferred(id: MarkID, position: CursorPosition = "start") {
    if (blockID.value && blockID.value !== id) {
      const prev = handles.get(blockID.value);
      prev?.blur();
    }
    blockID.value = id;
    pendingFocus = { id, intent: { type: "position", position } };
  }

  function blurAll() {
    if (blockID.value) {
      const handle = handles.get(blockID.value);
      handle?.blur();
      blockID.value = null;
    }
  }

  function isFocused(id: MarkID): boolean {
    return blockID.value === id;
  }

  return {
    blockID: readonly(blockID),
    groupID: readonly(groupID),
    registerEditor,
    unregisterEditor,
    getHandle,
    focusBlock,
    focusBlockDeferred,
    focusBlockAt,
    focusBlockWithRange,
    blurAll,
    isFocused,
  };
}

export type FocusContext = ReturnType<typeof useBlockFocus>;
