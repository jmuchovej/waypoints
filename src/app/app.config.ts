export default defineAppConfig({
  app: {
    name: "Waypoints",
  },
  ui: {
    colors: {
      primary: "cyan",
      neutral: "mist",
    },
    button: {
      slots: {
        base: "cursor-pointer",
      },
    },
    formField: {
      slots: {
        root: "w-full",
      },
    },
    input: {
      slots: {
        root: "w-full",
      },
    },
    textarea: {
      slots: {
        root: "w-full",
        base: "resize-none",
      },
    },
    scrollArea: {
      slots: {
        root: "scrollbar-none",
      },
    },
    dashboardGroup: {
      base: "relative flex flex-1 overflow-hidden",
    },
  },
});
