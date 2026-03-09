import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "2026-03-05",

  modules: [
    "@nuxt/eslint",
    "@nuxt/ui",
    "@nuxt/hints",
    "@nuxt/image",
    // "@nuxtjs/storybook",
    "@pinia/nuxt",
    "@vueuse/nuxt",
    "@nuxt/icon",
  ],

  devtools: {
    enabled: true,
  },

  css: ["~/assets/css/main.css"],
  components: [{ path: "~/components", pathPrefix: false }],

  routeRules: {
    "/": { prerender: true },
  },

  eslint: {
    config: {
      stylistic: {
        commaDangle: "always",
        braceStyle: "1tbs",
      },
    },
  },

  app: {
    head: {
      title: "Larder",
      charset: "utf-8",
      viewport: "width=device-width, initial-scale=1",
      meta: [
        { name: "format-detection", content: "no" },
        {
          name: "viewport",
          content:
            "width=device-width, initial-scale=1, maximum-scale=1, viewport-fit=cover",
        },
      ],
    },
    pageTransition: {
      name: "page",
      mode: "out-in",
    },
    layoutTransition: {
      name: "layout",
      mode: "out-in",
    },
  },
  css: ["@/assets/css/main.css"],
  svgo: {
    autoImportPath: "@/assets/",
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
  vite: {
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
    server: {
      strictPort: true,
      hmr: host
        ? {
          protocol: "ws",
          host,
          port: 3001,
        }
        : undefined,
    },
  },
  devServer: {
    host: host || "0",
    port: 9297,
  },
  router: {
    options: {
      scrollBehaviorType: "smooth",
    },
  },
  eslint: {
    config: {
      standalone: false,
    },
  },
  experimental: {
    typedPages: true,
  },
  // storybook: {
  //   url: "http://localhost:9298",
  //   storybookRoute: "/__storybook__",
  //   port: 9298,
  //   // Very verbose logs for debugging
  //   logLevel: Number.POSITIVE_INFINITY,
  // }
});
