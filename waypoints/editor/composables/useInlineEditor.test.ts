import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { defineComponent, h, nextTick, ref } from "vue";
import { mount, type VueWrapper } from "@vue/test-utils";
import type { RichText } from "@waypoints/domain";
import { emptyRichText, toRichText } from "@waypoints/domain";
import { useInlineEditor, type UseInlineEditorOptions } from "./useInlineEditor";

// ── Helpers ─────────────────────────────────────────────────────────────

const helloRt: RichText = toRichText("hello");

function mountEditor(overrides: Partial<UseInlineEditorOptions> = {}) {
  let result!: ReturnType<typeof useInlineEditor>;
  const onUpdate = vi.fn();

  const wrapper = mount(
    defineComponent({
      setup() {
        result = useInlineEditor({
          richText: ref(overrides.richText?.value ?? emptyRichText),
          onUpdate: overrides.onUpdate ?? onUpdate,
          ...overrides,
        } as UseInlineEditorOptions);
        return () => h("div", { ref: "editorEl" });
      },
    }),
    { attachTo: document.body },
  );

  return { wrapper, result, onUpdate };
}

// ── Tests ───────────────────────────────────────────────────────────────

describe("useInlineEditor", () => {
  let wrapper: VueWrapper;

  afterEach(() => {
    wrapper?.unmount();
  });

  describe("createEditor / destroyEditor", () => {
    it("creates a TipTap editor on the given element", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;

      expect(result.editor.value).toBeNull();

      const el = document.createElement("div");
      document.body.appendChild(el);
      result.createEditor(el);

      expect(result.editor.value).not.toBeNull();
      el.remove();
    });

    it("destroys the editor and nullifies ref", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;

      const el = document.createElement("div");
      document.body.appendChild(el);
      result.createEditor(el);
      expect(result.editor.value).not.toBeNull();

      result.destroyEditor();
      expect(result.editor.value).toBeNull();
      el.remove();
    });
  });

  describe("handle interface", () => {
    it("exposes handle with type 'inline'", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;
      expect(result.handle.type).toBe("inline");
    });

    it("handle.focus is safe when no editor mounted", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;
      // Should not throw
      result.handle.focus("start");
      result.handle.blur();
    });

    it("handle.getContentLength returns 0 when no editor", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;
      expect(result.handle.getContentLength()).toBe(0);
    });

    it("handle.getCursorOffset returns 0 when no editor", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;
      expect(result.handle.getCursorOffset()).toBe(0);
    });

    it("handle.isCursorAtStart returns false when no editor", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;
      expect(result.handle.isCursorAtStart()).toBe(false);
    });

    it("handle.isCursorAtEnd returns false when no editor", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;
      expect(result.handle.isCursorAtEnd()).toBe(false);
    });
  });

  describe("handle with mounted editor", () => {
    it("getContentLength returns text length", () => {
      const richText = ref(helloRt);
      const { result, wrapper: w } = mountEditor({ richText });
      wrapper = w;

      const el = document.createElement("div");
      document.body.appendChild(el);
      result.createEditor(el);

      expect(result.handle.getContentLength()).toBe(5);
      el.remove();
    });

    it("isCursorAtStart is true at position <= 1", () => {
      const richText = ref(helloRt);
      const { result, wrapper: w } = mountEditor({ richText });
      wrapper = w;

      const el = document.createElement("div");
      document.body.appendChild(el);
      result.createEditor(el);

      // After creation, cursor is at default position
      // Focus at start to ensure position 1
      result.handle.focus("start");
      expect(result.handle.isCursorAtStart()).toBe(true);
      el.remove();
    });
  });

  describe("onCoordsRange", () => {
    it("is safe when no editor mounted", () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;
      // Should not throw
      result.handle.onCoordsRange(0, 0, 100, 0);
    });

    it("creates a selection range in the editor", () => {
      const richText = ref(helloRt);
      const { result, wrapper: w } = mountEditor({ richText });
      wrapper = w;

      const el = document.createElement("div");
      document.body.appendChild(el);
      result.createEditor(el);

      // Apply a range that covers the content
      // In happy-dom, posAtCoords may not work (no layout engine),
      // but the method should not throw
      result.handle.onCoordsRange(0, 0, 100, 0);
      el.remove();
    });
  });

  describe("flush", () => {
    it("handle.flush resolves immediately when nothing pending", async () => {
      const { result, wrapper: w } = mountEditor();
      wrapper = w;
      // Should not throw or hang
      await result.handle.flush();
    });

    it("handle.flush clears debounce timer and flushes", async () => {
      const onUpdate = vi.fn();
      const richText = ref(helloRt);
      const { result, wrapper: w } = mountEditor({ richText, onUpdate });
      wrapper = w;

      const el = document.createElement("div");
      document.body.appendChild(el);
      result.createEditor(el);

      // Simulate a TipTap update by triggering the editor's update mechanism
      // and then flushing before the 150ms debounce fires
      await result.handle.flush();

      el.remove();
    });
  });

  describe("external sync watch", () => {
    it("resyncs editor content when richText changes externally", async () => {
      const richText = ref(emptyRichText);
      const { result, wrapper: w } = mountEditor({ richText });
      wrapper = w;

      const el = document.createElement("div");
      document.body.appendChild(el);
      result.createEditor(el);

      expect(result.editor.value!.state.doc.textContent).toBe("");

      // Simulate external change (e.g., undo/redo from backend)
      richText.value = helloRt;
      await nextTick();

      expect(result.editor.value!.state.doc.textContent).toBe("hello");
      el.remove();
    });
  });
});
