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
    dashboardSidebar: {
      slots: {
        // root: "transition-[left,right,width] duration-200 ease-linear",
      },
    },
    dashboardPanel: {
      slots: {
        // root: "transition-[left,right,width] duration-200 ease-linear",
      },
    },
    dashboardGroup: {
      base: "relative flex flex-1 overflow-hidden",
    },
    header: {
      slots: {
        content: "pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]",
      },
    },
    modal: {
      slots: {
        content: "pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]",
      },
    },
    slideover: {
      slots: {
        content: "pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]",
      },
    },
    drawer: {
      slots: {
        content: "pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]",
      },
    },
  },
});
