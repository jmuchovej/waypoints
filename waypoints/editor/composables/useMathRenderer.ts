import { ref } from "vue";

export type MathEngine = "katex" | "typst";

interface RenderResult {
  html: string;
  error?: string;
}

const renderCache = new Map<string, RenderResult>();

// Display-mode environment pattern (LaTeX)
const DISPLAY_MODE_RE = /\\begin\{/;

/** Rejects multi-line / display-mode expressions for inline use. */
function sanitizeForInline(expression: string): string | null {
  const trimmed = expression.trim();
  if (!trimmed) return null;
  // Reject display-mode LaTeX environments
  if (DISPLAY_MODE_RE.test(trimmed)) return null;
  // Strip newlines for inline constraint
  return trimmed.replace(/\n/g, " ");
}

let katexModule: typeof import("katex") | null = null;

async function loadKatex(): Promise<typeof import("katex")> {
  if (katexModule) return katexModule;
  katexModule = await import("katex");
  return katexModule;
}

async function renderKatex(expression: string): Promise<RenderResult> {
  const sanitized = sanitizeForInline(expression);
  if (!sanitized) {
    return { html: "", error: "Expression rejected for inline rendering" };
  }

  const cacheKey = `katex:${sanitized}`;
  const cached = renderCache.get(cacheKey);
  if (cached) return cached;

  try {
    const katex = await loadKatex();
    const html = katex.default.renderToString(sanitized, {
      displayMode: false,
      throwOnError: false,
    });
    const result: RenderResult = { html };
    renderCache.set(cacheKey, result);
    return result;
  } catch (e) {
    const result: RenderResult = { html: "", error: String(e) };
    return result;
  }
}

// Lazy-loaded typst.ts WASM compiler
type TypstSnippet = Awaited<ReturnType<typeof initTypst>>;
let typstReady: Promise<TypstSnippet> | null = null;

async function initTypst() {
  const { $typst } = await import("@myriaddreamin/typst.ts/contrib/all-in-one-lite");
  $typst.setCompilerInitOptions({
    getModule: () =>
      fetch(
        new URL(
          "@myriaddreamin/typst-ts-web-compiler/pkg/typst_ts_web_compiler_bg.wasm",
          import.meta.url,
        ),
      ).then((r) => r.arrayBuffer()),
  });
  $typst.setRendererInitOptions({
    getModule: () =>
      fetch(
        new URL("@myriaddreamin/typst-ts-renderer/pkg/typst_ts_renderer_bg.wasm", import.meta.url),
      ).then((r) => r.arrayBuffer()),
  });
  return $typst;
}

function loadTypst(): Promise<TypstSnippet> {
  if (!typstReady) typstReady = initTypst();
  return typstReady;
}

async function renderTypst(expression: string, displayMode = false): Promise<RenderResult> {
  const sanitized = displayMode ? expression.trim() : sanitizeForInline(expression);
  if (!sanitized) {
    return { html: "", error: "Expression rejected for rendering" };
  }

  const cacheKey = `typst:${displayMode ? "d" : "i"}:${sanitized}`;
  const cached = renderCache.get(cacheKey);
  if (cached) return cached;

  try {
    const $typst = await loadTypst();
    // Wrap expression in typst math delimiters
    const content = displayMode ? `$ ${sanitized} $` : `$${sanitized}$`;
    const svg = await $typst.svg({ mainContent: content });
    const result: RenderResult = { html: svg };
    renderCache.set(cacheKey, result);
    return result;
  } catch (e) {
    // Fallback to monospace display if WASM compilation fails
    const escaped = sanitized.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    return { html: `<code class="typst-fallback">${escaped}</code>`, error: String(e) };
  }
}

export function useMathRenderer() {
  const loading = ref(false);

  async function render(
    expression: string,
    engine: MathEngine = "katex",
    displayMode = false,
  ): Promise<RenderResult> {
    loading.value = true;
    try {
      if (engine === "typst") {
        return await renderTypst(expression, displayMode);
      }
      return await renderKatex(expression);
    } finally {
      loading.value = false;
    }
  }

  return { render, loading };
}
