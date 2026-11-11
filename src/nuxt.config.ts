// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { defineNuxtConfig } from "nuxt/config";
import { createViteLicensePlugin } from "rollup-license-plugin";

// The licenses a bundled package may carry. `cargo xtask license` holds this
// equal to `[licenses].allow` in `.config/deny.toml`: one list, two ecosystems.
const LICENSE_ALLOWLIST = [
  "MIT",
  "MIT-0",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "Unicode-3.0",
  "CC0-1.0",
  "MPL-2.0",
  "Unlicense",
  "0BSD",
  "BSL-1.0",
  "NCSA",
  "CDLA-Permissive-2.0",
];

// A package may declare an expression, and passes if some way of satisfying it
// stays on the list: `MIT OR EUPL-1.2` passes, `MIT AND EUPL-1.2` does not.
const licenseAllowed = (expression: string): boolean => {
  const flat = (text: string) =>
    text
      .split(" OR ")
      .some((alternative) =>
        alternative
          .split(" AND ")
          .every((term) => term === "true" || LICENSE_ALLOWLIST.includes(term)),
      );
  let reduced = expression;
  while (reduced.includes("(")) {
    reduced = reduced.replace(/\(([^()]*)\)/g, (_, inner: string) => String(flat(inner)));
  }
  return flat(reduced);
};

const licenseGate = createViteLicensePlugin({
  unacceptableLicenseTest: (expression) => !licenseAllowed(expression),
  licenseOverrides: {
    // Declares no license; upstream's LICENSE (unovue/vaul-vue) is MIT.
    "vaul-vue@0.4.1": "MIT",
  },
  outputFilename: false,
});

const licenseGatePlugin = {
  ...licenseGate,
  // The gate reads any module id containing `node_modules` as a package, and
  // Nuxt's virtual modules have such ids.
  renderChunk: (code: string, chunk: Parameters<typeof licenseGate.renderChunk>[1]) =>
    licenseGate.renderChunk(code, {
      ...chunk,
      modules: Object.fromEntries(
        Object.entries(chunk.modules).filter(([id]) => !id.startsWith("virtual:")),
      ),
    }),
};

const env = import.meta.env; // sugar
const host = env.NUXT_HMR_HOST || env.TAURI_DEV_HOST || undefined;
const isMobile = !!/android|ios/.exec(env.TAURI_ENV_PLATFORM || "");
const devPort = 9297;

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "2026-03-05",

  app: {
    head: {
      htmlAttrs: { lang: "en" },
      title: "Waypoints",
    },
  },

  modules: ["@nuxt/ui", "@nuxt/hints", "@nuxt/image", "@pinia/nuxt", "@vueuse/nuxt", "@nuxt/icon"],

  devtools: {
    enabled: true,
    timeline: {
      enabled: true,
    },
  },

  alias: {},

  css: ["~/assets/css/main.css", "~/assets/css/editor.css"],
  // components: [{ path: "~/components", pathPrefix: false }],

  ssr: false,

  // Dev-only: forward IPC calls from the Nuxt dev server (:9297) to the
  // `cargo xtask dev-server` HTTP backend, so the app runs in a plain browser
  // with HMR and same-origin `/ipc` requests. Has no effect on production
  // builds.
  nitro: {
    devProxy: {
      "/ipc": { target: "http://localhost:8123/ipc", changeOrigin: true },
    },
  },

  router: {
    options: {
      scrollBehaviorType: "smooth",
    },
  },

  ui: {
    fonts: true,
    mdc: true,
    content: true,
  },

  // https://nuxt.com/docs/4.x/guide/going-further/experimental-features
  experimental: {
    typedPages: true,
  },

  // mcp: {
  //   includeNuxtDocsMcp: true,
  // },

  // storybook: {
  //   url: "http://localhost:9298",
  //   storybookRoute: "/__storybook__",
  //   port: 9298,
  //   // Very verbose logs for debugging
  //   logLevel: Number.POSITIVE_INFINITY,
  // }

  sourcemap: {
    server: false,
    client: false,
  },

  vite: {
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
    plugins: [licenseGatePlugin],
    optimizeDeps: {
      include: [
        "@tiptap/core",
        "@tiptap/pm/state",
        "@tiptap/pm/view",
        "@tiptap/extension-paragraph",
        "@tiptap/extension-heading",
        "@tiptap/extension-code-block",
        "@tiptap/extension-blockquote",
        "@tiptap/extension-horizontal-rule",
        "@tiptap/vue-3",
        "@tiptap/vue-3/menus",
        "uuid",
        "@tauri-apps/api/core",
        "@tauri-apps/api/event",
        "@tauri-apps/api/webviewWindow",
        "@internationalized/date",
        "@tauri-apps/plugin-dialog",
        "@tiptap/extension-emoji",
        "katex",
        "@tiptap/extension-document",
        "@tiptap/extension-link",
        "@tiptap/extension-placeholder",
        "@tiptap/extension-text",
        "@tiptap/extension-bold",
        "@tiptap/extension-code",
        "@tiptap/extension-italic",
        "@tiptap/extension-strike",
        "@tiptap/extension-subscript",
        "@tiptap/extension-superscript",
        "@tiptap/extension-underline",
        "@vue/devtools-core",
        "@vue/devtools-kit",
        "@tauri-apps/api/window",
        "@myriaddreamin/typst.ts/contrib/all-in-one-lite",
        "@tiptap/extensions",
      ],
    },
    server: {
      strictPort: true,
      warmup: {},
      watch: {
        usePolling: true,
      },
    },
  },

  devServer: {
    host: isMobile ? "0.0.0.0" : undefined,
    port: devPort,
  },

  hooks: {
    // Monkey patch: https://github.com/tauri-apps/tauri/issues/11165#issuecomment-3694978357
    "vite:extend": ({ config }) => {
      if (isMobile && config.server?.hmr && config.server.hmr !== true) {
        config.server.hmr.port = devPort;
        config.server.hmr.protocol = "ws";
        config.server.hmr.host = host;
        config.server.origin = `http://${host}:${devPort}`;
      }
    },
  },
});
