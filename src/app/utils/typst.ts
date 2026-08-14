/**
 * Lazy singleton for the typst.ts WASM compiler, shared by every consumer
 * (math rendering, emergency-kit PDF export) so the app only ever
 * instantiates one compiler.
 */

export type TypstSnippet = Awaited<ReturnType<typeof initTypst>>;

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

export function loadTypst(): Promise<TypstSnippet> {
  if (!typstReady) typstReady = initTypst();
  return typstReady;
}
