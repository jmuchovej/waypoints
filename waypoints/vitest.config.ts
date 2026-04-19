import { defineConfig } from "vitest/config";
import { defineVitestProject } from "@nuxt/test-utils/config";

export default defineConfig({
  test: {
    projects: [
      // Layer 1: pure functions — no DOM, no Vue, no Nuxt
      {
        test: {
          name: "unit",
          include: ["editor/tiptap/**/*.test.ts"],
          environment: "node",
        },
      },
      // Layer 2+: composables & components — full Nuxt environment
      await defineVitestProject({
        test: {
          name: "nuxt",
          include: [
            "editor/composables/**/*.test.ts",
            "editor/components/**/*.test.ts",
            "app/**/*.test.ts",
          ],
          environment: "nuxt",
          environmentOptions: {
            nuxt: {
              domEnvironment: "happy-dom",
            },
          },
        },
      }),
    ],
  },
});
