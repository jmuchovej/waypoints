import type { PageCollections } from "@nuxt/content";
import { queryCollection } from "@nuxt/content/server";
// Imported explicitly (not via the toolkit's auto-import) so this file also
// type-checks while `mcp.enabled` is false and the auto-imports aren't wired.
import { defineMcpTool } from "@nuxtjs/mcp-toolkit/server";
import { joinURL } from "ufo";
import { z } from "zod";

/**
 * Shadows docus's get-page tool (docus/server/mcp/tools/get-page.ts, same
 * relative path — the mcp-toolkit loader gives project layers precedence),
 * which resolves every path against the `docs` collections. This version maps
 * `/docs/**` to the docs collection and everything else to the `pages`
 * (marketing) collection — the same split modules/routing.ts applies to
 * the app's routes.
 */
export default defineMcpTool({
  description: `Retrieves the full content and details of a specific site page — documentation or marketing.

WHEN TO USE: Use this tool when you know the EXACT path to a page. Common use cases:
- User asks for a specific page: "Show me the installation guide" → /docs/installation
- User asks about a known topic with a dedicated page: "Why was Waypoints built?" → /why
- You found a relevant path from list-pages and want the full content
- User references a specific section or guide they want to read

WHEN NOT TO USE: If you don't know the exact path and need to search/explore, use list-pages first.

WORKFLOW: This tool returns the complete page content including title, description, and full markdown. Use this when you need to provide detailed answers or code examples from specific pages.
`,
  annotations: {
    readOnlyHint: true,
    destructiveHint: false,
    idempotentHint: true,
    openWorldHint: false,
  },
  inputSchema: {
    path: z
      .string()
      .describe(
        "The page path from list-pages or provided by the user (e.g., /docs/installation, /why)",
      ),
  },
  inputExamples: [{ path: "/docs/installation" }, { path: "/why" }],
  cache: "1h",
  handler: async ({ path }) => {
    if (isNavigationPath(path)) {
      throw createError({ statusCode: 404, message: "Page not found" });
    }

    const event = useEvent();
    const config = useRuntimeConfig(event);
    const siteUrl =
      getRequestURL(event as unknown as Parameters<typeof getRequestURL>[0]).origin ||
      inferSiteURL();
    const baseURL = config.app?.baseURL || "/";

    const availableLocales = getAvailableLocales(config.public as Record<string, unknown>);
    const collectionName = getCollectionFromPath(path, availableLocales);

    try {
      // The event cast bridges the workspace's h3 v1/v2 type duality;
      // PageCollections (not Collections) keeps the roadmap data collection
      // out of the union so select() resolves.
      const page = await queryCollection(
        event as unknown as Parameters<typeof queryCollection>[0],
        collectionName as keyof PageCollections,
      )
        .where("path", "=", path)
        .select("title", "path", "description")
        .first();

      if (!page) {
        throw createError({ statusCode: 404, message: "Page not found" });
      }

      const content = await event.$fetch<string>(`/raw${path}.md`);

      return {
        title: page.title,
        path: page.path,
        description: page.description,
        content,
        url: siteUrl ? joinURL(siteUrl, baseURL, page.path) : joinURL(baseURL, page.path),
      };
    } catch (error) {
      if ((error as { statusCode?: number }).statusCode === 404) throw error;
      throw createError({ statusCode: 500, message: "Failed to get page" });
    }
  },
});

/**
 * `/docs/**` (or `/<locale>/docs/**`) → docs collection; everything else →
 * pages. Mirrors the route split in modules/routing.ts.
 */
function getCollectionFromPath(path: string, availableLocales: string[]): string {
  const [first, second] = path.split("/").filter(Boolean);
  const locale = first && availableLocales.includes(first) ? first : undefined;
  const isDocs = (locale ? second : first) === "docs";
  const base = isDocs ? "docs" : "pages";
  return locale ? `${base}_${locale}` : base;
}

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

function isNavigationPath(path: string): boolean {
  return path.endsWith(".navigation") || path.includes("/.navigation/");
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
