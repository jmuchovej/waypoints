import { TextSelection } from "@tiptap/pm/state";
import type { RichText } from "@waypoints/domain";
import {
  type Ref,
  type ShallowRef,
  inject,
  nextTick,
  onUnmounted,
  shallowReadonly,
  shallowRef,
  watch,
} from "vue";

import {
  createEditor as createBridgedEditor,
  mkDocument,
  toDomain,
  type Editor,
  type EditorOptions,
} from "~/utils/tiptap";

import type { CursorPosition, EditorHandle } from "./useBlockFocus";
import { BlockOrchestratorKey, type OrchestratorContext } from "./useBlockOrchestrator";
import { CommandMenuKey, type CommandMenuContext } from "./useCommandMenu";

export type UseInlineEditorOptions = Omit<EditorOptions, "richText" | "onUpdate"> & {
  richText: Ref<RichText>;
  onUpdate: (richText: RichText) => void | Promise<void>;
};

export function useInlineEditor(options: UseInlineEditorOptions) {
  const _orchestrator = inject<OrchestratorContext | undefined>(BlockOrchestratorKey, undefined);
  const _menu = inject<CommandMenuContext | undefined>(CommandMenuKey, undefined);

  const editor = shallowRef<Editor | null>(null);
  let debounceTimer = ref<DebounceTimer | null>(null);
  let lastUpdatePromise = ref<Promise<void> | null>(null);
  let isLocalUpdate = ref(false);

  function createEditor(element: HTMLElement) {
    editor.value = createBridgedEditor(element, {
      richText: options.richText.value,
      blockID: options.blockID,
      placeholder: options.placeholder,
      onKeyDown: options.onKeyDown,
      menu: _menu,
      orchestrator: _orchestrator,
      onUpdate: ({ editor }) => {
        if (debounceTimer.value) clearTimeout(debounceTimer.value);
        // tiptap types onUpdate's `editor` as the @tiptap/core Editor; the runtime
        // instance is the @tiptap/vue-3 subclass our bridge constructs.
        debounceTimer.value = setTimeout(() => flush(editor as Editor), 150);
      },
    });
  }

  function flush(editor: Editor) {
    flushUpdate(editor, isLocalUpdate, lastUpdatePromise, options.onUpdate);
  }

  function destroyEditor() {
    if (debounceTimer.value) {
      clearTimeout(debounceTimer.value);
      // Flush any pending update before destroying
      if (editor.value) {
        flush(editor.value);
      }
      debounceTimer.value = null;
    }
    editor.value?.destroy();
    editor.value = null;
  }

  const handle = createEditorHandle(
    editor,
    isLocalUpdate,
    debounceTimer,
    lastUpdatePromise,
    options.onUpdate,
  );

  // Resync TipTap when external changes arrive (undo/redo, merge, split).
  watch(options.richText, (newRt) => {
    if (isLocalUpdate.value || !editor.value) return;
    editor.value.commands.setContent(mkDocument(newRt));
  });

  onUnmounted(() => destroyEditor());

  return {
    editor: shallowReadonly(editor),
    handle,
    createEditor,
    destroyEditor,
  };
}

type DebounceTimer = ReturnType<typeof setTimeout>;

function createEditorHandle(
  editor: ShallowRef<Editor | null>,
  isLocalUpdate: Ref<boolean>,
  debounceTimer: Ref<DebounceTimer | null>,
  lastUpdatePromise: Ref<Promise<void> | null>,
  onUpdate: UseInlineEditorOptions["onUpdate"],
): EditorHandle {
  return {
    type: "inline",

    focus(position?: CursorPosition) {
      editor.value?.commands.focus(position === "start" ? "start" : "end");
    },

    blur() {
      editor.value?.commands.blur();
    },

    onCoords(x: number, y: number) {
      const ed = editor.value;
      if (!ed) return;
      const pos = ed.view.posAtCoords({ left: x, top: y });
      if (pos) {
        ed.commands.focus(pos.pos);
      } else {
        ed.commands.focus("end");
      }
    },

    onCoordsRange(x1: number, y1: number, x2: number, y2: number) {
      const ed = editor.value;
      if (!ed) return;
      const start = ed.view.posAtCoords({ left: x1, top: y1 });
      const end = ed.view.posAtCoords({ left: x2, top: y2 });
      if (start && end) {
        const sel = TextSelection.create(ed.state.doc, start.pos, end.pos);
        ed.view.dispatch(ed.state.tr.setSelection(sel));
        ed.view.focus();
      } else {
        ed.commands.focus(start?.pos ?? "end");
      }
    },

    getCursorOffset(): number {
      return editor.value?.state.selection.$head.pos ?? 0;
    },

    isCursorAtStart(): boolean {
      const ed = editor.value;
      if (!ed) return false;
      // Position 1 = start of content inside the paragraph (pos 0 is before doc)
      return ed.state.selection.$head.pos <= 1;
    },

    isCursorAtEnd(): boolean {
      const ed = editor.value;
      if (!ed) return false;
      const docSize = ed.state.doc.content.size;
      return ed.state.selection.$head.pos >= docSize - 1;
    },

    isCursorOnFirstLine(): boolean {
      const ed = editor.value;
      if (!ed) return true;
      try {
        const cur = ed.view.coordsAtPos(ed.state.selection.from);
        const start = ed.view.coordsAtPos(1);
        return Math.abs(cur.top - start.top) < 2;
      } catch {
        // coordsAtPos can fail if view isn't laid out; fall back to position check
        return ed.state.selection.$head.pos <= 1;
      }
    },

    isCursorOnLastLine(): boolean {
      const ed = editor.value;
      if (!ed) return true;
      const docSize = ed.state.doc.content.size;
      try {
        const cur = ed.view.coordsAtPos(ed.state.selection.from);
        const end = ed.view.coordsAtPos(Math.max(1, docSize - 1));
        return Math.abs(cur.top - end.top) < 2;
      } catch {
        return ed.state.selection.$head.pos >= docSize - 1;
      }
    },

    getContentLength(): number {
      const ed = editor.value;
      if (!ed) return 0;
      return ed.state.doc.textContent.length;
    },

    async flush() {
      if (debounceTimer.value) {
        clearTimeout(debounceTimer.value);
        debounceTimer.value = null;
      }
      if (editor.value) {
        flushUpdate(editor.value, isLocalUpdate, lastUpdatePromise, onUpdate);
      }
      if (lastUpdatePromise.value) {
        await lastUpdatePromise.value;
        lastUpdatePromise.value = null;
      }
    },

    canUndo(): boolean {
      return editor.value?.can().undo() ?? false;
    },

    canRedo(): boolean {
      return editor.value?.can().redo() ?? false;
    },

    undo() {
      editor.value?.commands.undo();
    },

    redo() {
      editor.value?.commands.redo();
    },
  };
}

function flushUpdate(
  editor: Editor,
  isLocalUpdate: Ref<boolean>,
  lastUpdatePromise: Ref<Promise<void> | null>,
  onUpdate: UseInlineEditorOptions["onUpdate"],
) {
  const doc = editor.getJSON();
  const inner = doc.content?.[0]?.content ?? [];
  const richText = toDomain(inner);
  isLocalUpdate.value = true;
  const result = onUpdate(richText);
  if (result instanceof Promise) {
    lastUpdatePromise.value = result;
    // Keep the guard up until the backend response propagates through reactivity
    result.then(() => {
      nextTick(() => (isLocalUpdate.value = false));
    });
  } else {
    nextTick(() => (isLocalUpdate.value = false));
  }
}
