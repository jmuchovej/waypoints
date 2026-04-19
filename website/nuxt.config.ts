import { defineNuxtConfig } from "nuxt/config";

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  extends: ["docus"],

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
    "/releases": { redirect: "/releases/stable" },
    "/releases/**": { prerender: true, experimentalNoScripts: false },
    "/api/v1/update/**": { prerender: true },
  },

  experimental: {
    payloadExtraction: false,
  },

  vite: {
    optimizeDeps: {
      include: ["@vue/devtools-core", "@vue/devtools-kit", "tailwindcss/colors"],
    },
  },

  compatibilityDate: "2025-01-15",

  hints: {
    devtools: true,
  },

  site: {
    name: "Waypoints",
  },
});
