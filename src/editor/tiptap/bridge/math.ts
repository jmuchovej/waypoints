import { getMarkRange, Mark, markInputRule, markPasteRule, mergeAttributes } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import { InlineKind } from "@waypoints/domain";

const inputRegex = /(?:^|\s)(\$([^$]+)\$)$/;
const pasteRegex = /(?:^|\s)(\$([^$]+)\$)/g;

const name = "math";
const key = new PluginKey(name);

let renderTimer: ReturnType<typeof setTimeout> | null = null;
let cachedSvg: Map<string, string> = new Map();

async function renderMathPreview(expression: string, engine: string): Promise<string> {
  const cacheKey = `${engine}:${expression}`;
  const cached = cachedSvg.get(cacheKey);
  if (cached) return cached;

  try {
    if (engine === "katex" || engine === "latex") {
      const katex = await import("katex");
      const html = katex.default.renderToString(expression, {
        displayMode: false,
        throwOnError: false,
      });
      cachedSvg.set(cacheKey, html);
      return html;
    }

    // For typst, try Tauri IPC if available
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const svg = await invoke<string>("editor_render_math", {
        expression,
        engine: "typst",
        displayMode: false,
      });
      cachedSvg.set(cacheKey, svg);
      return svg;
    } catch {
      // Fallback to monospace display
      const escaped = expression.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
      const html = `<code class="typst-fallback">${escaped}</code>`;
      cachedSvg.set(cacheKey, html);
      return html;
    }
  } catch {
    return "";
  }
}

export const Math = Mark.create({
  name: InlineKind.Math,

  addAttributes() {
    return {
      engine: {
        default: "typst",
        parseHTML: (el: HTMLElement) => el.getAttribute("data-engine") ?? "typst",
        renderHTML: (attrs: Record<string, unknown>) => ({
          "data-engine": attrs.engine ?? "typst",
        }),
      },
    };
  },

  parseHTML() {
    return [{ tag: `span[data-inline="${InlineKind.Math}"]` }];
  },

  renderHTML({ HTMLAttributes }) {
    return ["span", mergeAttributes(HTMLAttributes, { "data-inline": InlineKind.Math }), 0];
  },

  addKeyboardShortcuts() {
    return {
      "Mod-Shift-m": () => this.editor.commands.toggleMark(this.name),
    };
  },

  addInputRules() {
    return [markInputRule({ find: inputRegex, type: this.type })];
  },

  addPasteRules() {
    return [markPasteRule({ find: pasteRegex, type: this.type })];
  },

  addProseMirrorPlugins() {
    const markType = this.type;

    return [
      new Plugin({
        key,
        state: {
          init() {
            return DecorationSet.empty;
          },

          apply(tr, old, _oldState, newState) {
            const { $from } = newState.selection;
            const range = getMarkRange($from, markType);

            if (!range) return DecorationSet.empty;

            // Show the raw text when editing (active mark styling)
            const inlineDecoration = Decoration.inline(range.from, range.to, {
              class: "math-editing",
            });

            // Extract expression text for the preview widget
            const expression = newState.doc.textBetween(range.from, range.to);
            const engine = $from.marks().find((m) => m.type === markType)?.attrs?.engine ?? "typst";

            const widget = Decoration.widget(
              range.to,
              () => {
                const preview = document.createElement("span");
                preview.className = "math-preview";
                preview.contentEditable = "false";

                // Debounced render
                if (renderTimer) clearTimeout(renderTimer);
                renderTimer = setTimeout(() => {
                  renderMathPreview(expression, engine).then((html) => {
                    if (preview.isConnected) {
                      preview.innerHTML = html;
                    }
                  });
                }, 200);

                return preview;
              },
              { side: 1 },
            );

            return DecorationSet.create(newState.doc, [inlineDecoration, widget]);
          },
        },

        props: {
          decorations(state) {
            return key.getState(state);
          },
        },
      }),
    ];
  },
});
