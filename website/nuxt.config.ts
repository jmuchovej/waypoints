import { defineNuxtConfig } from "nuxt/config";
import { loadSopsSecrets } from "./secrets";

await loadSopsSecrets();

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
    // App/site association (served by the /.well-known routes). Override
    // per-environment via NUXT_APP_ASSOCIATION_* env vars — the Android
    // fingerprints stay empty until the release signing cert is pinned.
    appAssociation: {
      appleTeamId: "D6NBHNJ93R",
      // Comma-separated SHA-256 cert fingerprints.
      androidCertFingerprints: "",
    },
    turso: {
      databaseUrl: process.env.TURSO_DATABASE_URL,
      authToken: process.env.TURSO_AUTH_TOKEN,
    },
    public: {
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

  mcp: {
    enabled: false,
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

  llms: {
    domain: "https://waypoints.so",
    title: "Waypoints",
    description: "",
    full: {
      title: "Waypoints",
      description: "",
    },
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
