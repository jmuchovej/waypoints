import type { DefinedCollection } from "@nuxt/content";
import "@nuxtjs/i18n";
import { defineCollection, defineContentConfig, z } from "@nuxt/content";
import { useNuxt } from "@nuxt/kit";
import { defineSitemapSchema } from "@nuxtjs/sitemap/content";
import { joinURL, withLeadingSlash, withoutLeadingSlash } from "ufo";

const { options } = useNuxt();
const cwd = joinURL(options.rootDir, "content");
const locales = options.i18n?.locales;

const collections: Record<string, DefinedCollection> = {};

const definePageCollection = (name: string, code?: string) => {
  const prefix = code ? code : "";
  return defineCollection({
    type: "page",
    source: {
      cwd,
      prefix: withLeadingSlash(prefix),
      include: withoutLeadingSlash(`${prefix}/**/*.md`),
      exclude: [withoutLeadingSlash(`${prefix}/docs/**`)],
    },
    schema: z.object({
      title: z.string(),
      description: z.string().optional(),
    }),
  });
};

if (locales && Array.isArray(locales)) {
  for (const locale of locales) {
    const code = (typeof locale === "string" ? locale : locale.code).replace("-", "_");
    const name = `pages_${code}`;
    collections[name] = definePageCollection(name, code);
  }
} else {
  collections[`pages`] = definePageCollection("pages");
}

collections[`roadmap`] = defineCollection({
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
});

export default defineContentConfig({ collections });
