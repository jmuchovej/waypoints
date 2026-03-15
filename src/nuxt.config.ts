import process from "node:process";
import { defineNuxtConfig } from "nuxt/config";

const host = process.env.TAURI_DEV_HOST;

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "2026-03-05",

  modules: [
    "@nuxt/ui",
    "@nuxt/hints",
    "@nuxt/image",
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

  app: {
    head: {
      title: "Waypoints",
      charset: "utf-8",
      viewport: "width=device-width, initial-scale=1",
      meta: [
        { name: "format-detection", content: "no" },
        {
          name: "viewport",
          content: "width=device-width, initial-scale=1, maximum-scale=1, viewport-fit=cover",
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

  ui: {
    fonts: true,
    mdc: true,
    content: true,
  },

  // https://nuxt.com/docs/4.x/guide/going-further/experimental-features
  experimental: {
    typedPages: true,
  },
});
