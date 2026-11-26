// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { defineContentConfig, defineCollection, z } from "@nuxt/content";

export default defineContentConfig({
  collections: {
    pages: defineCollection({
      type: "page",
      source: {
        include: "*.md",
        exclude: ["index.md"],
      },
    }),
    roadmap: defineCollection({
      type: "data",
      source: "roadmap.yaml",
      schema: z.object({
        items: z.array(
          z.object({
            title: z.string(),
            description: z.string(),
            status: z.enum(["done", "in-progress", "next", "later"]),
          }),
        ),
      }),
    }),
  },
});
