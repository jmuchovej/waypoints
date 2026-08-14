import { defineNuxtConfig } from "nuxt/config";

const URL = "https://waypoints.so";
const NAME = "Waypoints";
const PORT = 9732;

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: "2026-08-13",

  app: {
    baseURL: "/",
    head: {
      titleTemplate: "%s | %siteName",
      templateParams: {
        siteName: NAME,
      },
    },
  },

  devServer: {
    port: PORT,
  },

  ssr: true,

  $development: {
    debug: true,
    devtools: {
      enabled: true,
    },
    hints: {
      devtools: true,
    },
    site: {
      url: `http://localhost:${PORT}`,
      debug: true,
    },
  },

  $production: {
    site: {
      url: URL,
    },
    devtools: {
      enabled: false,
    },
  },

  extends: ["docus", "nuxtseo-layer-devtools"],

  // Docus provides: @nuxt/ui, @nuxt/content, @nuxt/image, @nuxtjs/mcp-toolkit,
  // @nuxtjs/robots, nuxt-og-image, nuxt-llms
  modules: [
    "@nuxtjs/sitemap",
    // "@nuxt/eslint",
    // "@nuxt/a11y",
    "@nuxt/hints",
    "@nuxt/scripts",
    // "@nuxtjs/plausible",
    "@nuxt/test-utils/module",
    // "@pinia/nuxt",
    "@nuxtjs/turnstile",
    "@vueuse/nuxt",
    "@nuxt/fonts",
    // "@nuxtjs/i18n",
    "nuxt-schema-org",
    "nuxt-link-checker",
    "nuxt-site-config",
  ],

  site: {
    name: NAME,
    trailingSlash: true,
    defaultLocale: "en",
    debug: true,
  },

  colorMode: {
    preference: "system",
  },

  fonts: {
    priority: ["npm", "fontsource", "bunny"],
    providers: {
      google: false,
      googleicons: false,
      fontsource: false,
      adobe: false,
    },
    npm: {
      remote: false,
    },
    defaults: {
      weights: ["400 700"],
      styles: ["normal", "italic"],
      subsets: ["latin", "latin-ext"],
    },
    families: [
      { name: "Albert Sans Variable", provider: "npm" },
      { name: "Monaspace Argon", provider: "npm" },
    ],
    experimental: {
      disableLocalFallbacks: true,
    },
  },

  // i18n: {
  //   baseUrl: URL,
  //   strategy: "prefix_and_default",
  //   locales: [
  //     { code: "en", name: "English (United States)", language: "en-US" },
  //     { code: "pt", name: "Português (Brasil)", language: "pt-BR" },
  //   ],
  //   customRoutes: "meta",
  //   defaultLocale: "en",
  //   detectBrowserLanguage: {
  //     fallbackLocale: "en",
  //     useCookie: true,
  //   },
  //   customBlocks: {
  //     defaultSFCLang: "yaml",
  //   },
  //   experimental: {
  //     compactRoutes: true,
  //   },
  // },

  icon: {
    clientBundle: {
      icons: ["simple-icons:discord"],
      scan: true,
    },
  },

  llms: {
    domain: URL,
    title: NAME,
    description: "",
    full: {
      title: NAME,
      description: "",
    },
  },

  mcp: {
    enabled: false,
  },

  ogImage: {
    enabled: true,
  },

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
      databaseUrl: "",
      authToken: "",
    },
    public: {
      github: {
        appId: "",
      },
    },
  },

  routeRules: {
    // "/": { prerender: true },
    "/download": { redirect: "/download/stable" },
    "/download/**": { prerender: true },
    "/releases": { redirect: "/releases/stable" },
    "/releases/**": { prerender: true, experimentalNoScripts: false },
    "/api/v1/update/**": { prerender: true },
  },

  css: ["~/assets/css/main.css"],

  experimental: {
    payloadExtraction: false,
    defaults: {
      nuxtLink: {
        trailingSlash: "append",
      },
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
        // "elkjs/lib/elk.bundled.js",
      ],
    },
  },

  nitro: {
    preset: "cloudflare_module",
    cloudflare: {
      deployConfig: true,
      nodeCompat: true,
    },
    prerender: {
      autoSubfolderIndex: true,
      crawlLinks: true,
    },
  },

  schemaOrg: {
    identity: {
      type: "Organization",
      name: "Waypoints",
      logo: "/favicon.svg",
      sameAs: ["https://github.com/waypoints-hq", "https://bsky.app/profile/waypoints.so"],
    },
  },
});
