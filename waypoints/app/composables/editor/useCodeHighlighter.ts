import { createHighlighter, type Highlighter, type BundledLanguage } from "shiki";
import { ref, watch, type Ref } from "vue";

let highlighterPromise: Promise<Highlighter> | null = null;

function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({
      themes: ["github-light", "github-dark"],
      langs: [],
    });
  }
  return highlighterPromise;
}

export function useCodeHighlighter(code: Ref<string>, language: Ref<string>) {
  const html = ref("");
  const loading = ref(true);

  async function highlight() {
    const raw = code.value;
    const lang = language.value || "text";

    if (!raw) {
      html.value = "";
      loading.value = false;
      return;
    }

    try {
      const highlighter = await getHighlighter();

      // Lazily load the language if not already loaded
      const loadedLangs = highlighter.getLoadedLanguages();
      if (!loadedLangs.includes(lang as BundledLanguage)) {
        try {
          await highlighter.loadLanguage(lang as BundledLanguage);
        } catch {
          // Unknown language — fall back to plaintext
          if (!loadedLangs.includes("text")) {
            await highlighter.loadLanguage("text");
          }
        }
      }

      const effectiveLang = highlighter.getLoadedLanguages().includes(lang as BundledLanguage)
        ? lang
        : "text";

      html.value = highlighter.codeToHtml(raw, {
        lang: effectiveLang,
        themes: {
          light: "github-light",
          dark: "github-dark",
        },
      });
    } catch {
      // Fallback: render as plain text
      const escaped = raw.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
      html.value = `<pre><code>${escaped}</code></pre>`;
    } finally {
      loading.value = false;
    }
  }

  watch([code, language], highlight, { immediate: true });

  return { html, loading };
}
