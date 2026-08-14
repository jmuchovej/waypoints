import type { PageCollections } from "@nuxt/content";
import { queryCollection } from "@nuxt/content/server";
// Imported explicitly (not via the toolkit's auto-import) so this file also
// type-checks while `mcp.enabled` is false and the auto-imports aren't wired.
import { defineMcpTool } from "@nuxtjs/mcp-toolkit/server";
import { joinURL } from "ufo";
import { z } from "zod";

/**
 * Shadows docus's list-pages tool (docus/server/mcp/tools/list-pages.ts, same
 * relative path — the mcp-toolkit loader gives project layers precedence),
 * which only enumerates `docs_*` collections. This version walks the `pages`
 * (marketing) collections too. Self-contained for the same reason as
 * server/routes/sitemap.xml.ts.
 */
export default defineMcpTool({
  description: `Lists all available site pages — documentation and marketing — with their basic information.

WHEN TO USE: Use this tool when you need to EXPLORE or SEARCH for content about a topic but don't know the exact page path. Common scenarios:
- "Find documentation about markdown features" - explore available guides
- "Show me all getting started guides" - browse introductory content
- "What does the site say about Obsidian?" - marketing comparisons live here too
- User asks general questions without specifying exact pages
- You need to understand the overall site structure

WHEN NOT TO USE: If you already know the specific page path (e.g., "/docs/installation"), use get-page directly instead.

WORKFLOW: This tool returns page titles, descriptions, and paths. After finding relevant pages, use get-page to retrieve the full content of specific pages that match the user's needs.

OUTPUT: Returns a structured list with:
- title: Human-readable page name
- path: Exact path for use with get-page
- description: Brief summary of page content
- section: "docs" (documentation) or "pages" (marketing site)
- url: Full URL for reference`,
  annotations: {
    readOnlyHint: true,
    destructiveHint: false,
    idempotentHint: true,
    openWorldHint: false,
  },
  inputSchema: {
    locale: z.string().optional().describe('The locale to filter pages by (e.g., "en", "pt")'),
  },
  inputExamples: [{ locale: "en" }, {}],
  cache: "1h",
  handler: async ({ locale }) => {
    const event = useEvent();
    const config = useRuntimeConfig(event);

    const siteUrl =
      getRequestURL(event as unknown as Parameters<typeof getRequestURL>[0]).origin ||
      inferSiteURL();
    const baseURL = config.app?.baseURL || "/";
    const availableLocales = getAvailableLocales(config.public as Record<string, unknown>);

    const locales = locale && availableLocales.includes(locale) ? [locale] : availableLocales;
    const collections: Array<{ name: string; section: "docs" | "pages"; locale?: string }> = (
      locales.length > 0 ? locales : [undefined]
    ).flatMap((code) => [
      { name: code ? `docs_${code}` : "docs", section: "docs" as const, locale: code },
      { name: code ? `pages_${code}` : "pages", section: "pages" as const, locale: code },
    ]);

    try {
      const allPages = await Promise.all(
        collections.map(async ({ name, section, locale: code }) => {
          // The event cast bridges the workspace's h3 v1/v2 type duality;
          // PageCollections (not Collections) keeps the roadmap data
          // collection out of the union so select() resolves.
          const pages = await queryCollection(
            event as unknown as Parameters<typeof queryCollection>[0],
            name as keyof PageCollections,
          )
            .where("path", "NOT LIKE", "%.navigation")
            .select("title", "path", "description")
            .all();

          return pages.map((page) => ({
            title: page.title,
            path: page.path,
            description: page.description,
            section,
            ...(code ? { locale: code } : {}),
            url: siteUrl ? joinURL(siteUrl, baseURL, page.path) : joinURL(baseURL, page.path),
          }));
        }),
      );

      return allPages.flat();
    } catch {
      throw createError({ statusCode: 500, message: "Failed to list pages" });
    }
  },
});

type ConfigWithLocales = {
  i18n?: { locales?: Array<string | { code: string }> };
  docus?: { filteredLocales?: Array<{ code: string }> };
};

function getAvailableLocales(config: ConfigWithLocales): string[] {
  if (config.docus?.filteredLocales) {
    return config.docus.filteredLocales.map((locale) => locale.code);
  }

  return config.i18n?.locales
    ? config.i18n.locales.map((locale) => (typeof locale === "string" ? locale : locale.code))
    : [];
}

// Same env chain docus uses (docus/utils/meta.ts).
function inferSiteURL(): string | undefined {
  return (
    process.env.NUXT_PUBLIC_SITE_URL ||
    process.env.NUXT_SITE_URL ||
    process.env.VERCEL_PROJECT_PRODUCTION_URL ||
    process.env.VERCEL_BRANCH_URL ||
    process.env.VERCEL_URL ||
    process.env.URL ||
    process.env.CI_PAGES_URL ||
    process.env.CF_PAGES_URL
  );
}
