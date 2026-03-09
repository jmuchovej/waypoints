export default defineAppConfig({
  app: {
    name: "Larder",
  },
  ui: {
    colors: {
      primary: "green",
      neutral: "zinc",
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
  },
});
