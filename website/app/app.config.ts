export default defineAppConfig({
  repository: "waypoints-hq/waypoints",
  app: {
    rootAttrs: {
      "data-vaul-drawer-wrapper": "",
    },
  },
  ui: {
    colors: {
      primary: "indigo",
      secondary: "sky",
      neutral: "mist",
      success: "emerald",
      warning: "amber",
      info: "orange",
      error: "rose",
    },
    page: {
      slots: {
        root: "max-w-(--ui-container) mx-auto",
      },
    },
    footer: {
      slots: {
        root: "md:border-t border-default",
      },
    },
    button: {
      defaultVariants: {
        variant: "subtle",
      },
    },
    kbd: {
      base: "font-bold font-mono",
      variants: {
        size: {
          sm: "leading-4",
          md: "leading-5",
          lg: "leading-6",
        },
      },
      defaultVariants: {
        variant: "subtle",
      },
    },
  },
  seo: {
    titleTemplate: "%s | Waypoints",
    title: "Waypoints",
    description: "",
  },
  header: {
    title: "Waypoints",
    logo: {
      light: "/logo/logo-light-md.svg",
      dark: "/logo/logo-dark-md.svg",
      alt: "The Waypoints Logo",
      class: "h-10",
      wordmark: {
        light: "/logo/wordmark-light-md.svg",
        dark: "/logo/wordmark-dark-md.svg",
      },
      favicon: "/favicon.svg",
      brandAssetsUrl: "https://waypoints.so/brand",
    },
  },
  socials: {
    discord: "https://discord.gg/kMrCfcY4R5",
  },
});
