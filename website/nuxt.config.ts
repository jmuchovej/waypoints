import { defineNuxtConfig } from "nuxt/config";
import { loadSopsSecrets } from "./secrets";

await loadSopsSecrets();
console.log(process.env.NUXT_TURSO_DATABASE_URL);

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  extends: ["docus"],

  runtimeConfig: {
    github: {
      token: "",
      clientId: "",
      clientSecret: "",
    },
    betterAuthSecret: "",
    turso: {
      databaseUrl: process.env.TURSO_DATABASE_URL,
      authToken: process.env.TURSO_AUTH_TOKEN,
    },
    public: {
      turnstileSiteKey: "",
      github: {
        appId: "",
      },
    },
  },

  // Docus provides: @nuxt/ui, @nuxt/content, @nuxt/image, @nuxtjs/mcp-toolkit,
  // @nuxtjs/robots, nuxt-og-image, nuxt-llms
  modules: [
    // "@nuxt/eslint",
    // "@nuxt/a11y",
    // "@nuxt/hints",
    "@nuxt/scripts",
    "@nuxt/test-utils",
    // "@nuxtjs/plausible",
    "@nuxtjs/turnstile",
    // "@pinia/nuxt",
    "@vueuse/nuxt",
  ],

  devtools: {
    enabled: true,
  },

  css: ["~/assets/css/main.css"],

  routeRules: {
    "/": { prerender: true },
    "/download": { redirect: "/download/stable" },
    "/download/**": { prerender: true },
    "/releases": { redirect: "/releases/stable" },
    "/releases/**": { prerender: true, experimentalNoScripts: false },
    "/api/v1/update/**": { prerender: true },
  },

  experimental: {
    payloadExtraction: false,
  },

  colorMode: {
    preference: "system",
  },

  compatibilityDate: "2025-01-15",

  // hints: {
  //   devtools: true,
  // },

  site: {
    name: "Waypoints",
  },

  vite: {
    optimizeDeps: {
      include: [
        "@vue/devtools-core",
        "@vue/devtools-kit",
        "tailwindcss/colors",
        "better-auth/vue",
        "better-auth/client/plugins",
        "@better-auth/passkey/client",
      ],
    },
  },
});
