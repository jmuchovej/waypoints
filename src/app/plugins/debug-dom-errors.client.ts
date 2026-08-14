/**
 * Dev-only diagnostic plugin — enriches Vue DOM errors with component trace,
 * props, and editor element state. Uses the `vue:error` hook so Vue's default
 * error handling (crash + console warning) is preserved.
 *
 * Stripped from production builds via `import.meta.dev` guard.
 */
import { createLogger } from "~/utils/logger";

const log = createLogger("dom-debug");

export default defineNuxtPlugin((nuxtApp) => {
  if (!import.meta.dev) return;

  nuxtApp.hook("vue:error", (err, instance, info) => {
    const error = err as Error;
    if (
      !error?.message?.includes("insertBefore") &&
      !error?.message?.includes("subTree") &&
      !error?.message?.includes("nextSibling")
    ) {
      return;
    }

    log.error("Vue renderer error:", error.message);
    log.error("Info:", info);

    if (instance) {
      const chain: string[] = [];
      let current: typeof instance | null = instance;
      while (current) {
        const name = current.$options?.name || current.$options?.__name || "Anonymous";
        const props = current.$props ? Object.keys(current.$props) : [];
        chain.push(`${name}(${props.join(", ")})`);
        current = current.$parent;
      }
      log.error("Component chain (child → root):", chain.join(" → "));

      if (instance.$props) {
        log.error(
          "Props:",
          JSON.parse(
            JSON.stringify(instance.$props, (_, v) =>
              v instanceof HTMLElement
                ? `<${v.tagName.toLowerCase()}>`
                : typeof v === "object" && v !== null && "state" in v
                  ? "[TipTap Editor]"
                  : v,
            ),
          ),
        );
      }

      if ("editorEl" in instance) {
        const el = (instance as any).editorEl;
        log.error("editorEl:", el);
        log.error("editorEl.parentNode:", el?.parentNode);
        log.error("editorEl in document:", el ? document.contains(el) : "null");
      }
    }

    log.error("Stack:", error.stack);
  });
});
