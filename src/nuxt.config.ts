import { defineNuxtConfig } from "nuxt/config";

const host = import.meta.env.NUXT_HMR_HOST || import.meta.env.TAURI_DEV_HOST || undefined;
const isMobile = !!/android|ios/.exec(import.meta.env.TAURI_ENV_PLATFORM || "");
const devPort = 9297;

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "2026-03-05",

  modules: ["@nuxt/ui", "@nuxt/hints", "@nuxt/image", "@pinia/nuxt", "@vueuse/nuxt", "@nuxt/icon"],

  devtools: {
    enabled: true,
  },

  css: ["~/assets/css/main.css", "~editor/assets/editor.css"],
  components: [{ path: "~/components", pathPrefix: false }],

  },

  ssr: false,

  dir: {
    modules: "app/modules",
  },

  imports: {
    presets: [
      {
        from: "zod",
        imports: [
          "z",
          {
            name: "infer",
            as: "zInfer",
            type: true,
          },
        ],
      },
    ],
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

  mcp: {
    includeNuxtDocsMcp: true,
  },

  // storybook: {
  //   url: "http://localhost:9298",
  //   storybookRoute: "/__storybook__",
  //   port: 9298,
  //   // Very verbose logs for debugging
  //   logLevel: Number.POSITIVE_INFINITY,
  // }

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
        "@tiptap/extension-list-item",
        "@tiptap/extension-blockquote",
        "@tiptap/extension-horizontal-rule",
        "@tiptap/vue-3",
        "@tiptap/vue-3/menus",
        "@tiptap/starter-kit",
        "uuid",
        "zod",
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
