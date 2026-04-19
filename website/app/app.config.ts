export default defineAppConfig({
  repository: "waypoints-hq/waypoints",
  ui: {
    colors: {
      primary: "sky",
      neutral: "mist",
    },
  },
  seo: {
    titleTemplate: "%s | %siteName",
    title: "Waypoints",
    description: "",
  },
  header: {
    title: "Waypoints",
    logo: {
      light: "/logo/logo-light-md.svg",
      dark: "/logo/logo-dark-md.svg",
      alt: "The Waypoints Logo",
      class: "h-8",
      wordmark: {
        light: "/logo/wordmark-light-md.svg",
        dark: "/logo/wordmark-dark-md.svg",
      },
      favicon: "/favicon.svg",
      brandAssetsUrl: "https://waypoints.so/brand",
    },
  },
  socials: {
    discord: "https://discord.com/invite/ps2h6QT",
    github: "https://github.com/waypoints-hq/waypoints",
  },
});
