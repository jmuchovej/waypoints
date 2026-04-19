import { defineNuxtConfig } from "nuxt/config";

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

  alias: {
    "~editor": "../editor",
  },

  css: ["~/assets/css/main.css", "~editor/assets/editor.css"],
  components: [{ path: "~/components", pathPrefix: false }],

  typescript: {
    tsConfig: {
      include: ["../editor/**/*"],
    },
  },

  ssr: false,

  dir: {
    modules: "app/modules",
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
        ignored: ["**/src-tauri/**"],
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
