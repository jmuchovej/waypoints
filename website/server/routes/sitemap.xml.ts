import { queryCollection } from "@nuxt/content/server";

/**
 * Shadows docus's sitemap route (docus/server/routes/sitemap.xml.ts), which
 * only enumerates its own `docs_*` (+ the patched-away `landing_*`)
 * collections — marketing pages were invisible. This version walks the
 * `pages` collections too. Deliberately self-contained: docus's server utils
 * are private to the layer, and the few lines they'd save aren't worth the
 * coupling.
 */

interface SitemapUrl {
  loc: string;
  lastmod?: string;
}

export default defineEventHandler(async (event) => {
  // Bridges the workspace's h3 v1/v2 type duality (nitro infers a v1 event
  // here; useRuntimeConfig expects v2) — runtime is unaffected.
  const config = useRuntimeConfig(event as unknown as Parameters<typeof useRuntimeConfig>[0]);
  const siteUrl = inferSiteURL() || "";

  const locales = getAvailableLocales(config.public as Record<string, unknown>);
  const collections =
    locales.length > 0
      ? locales.flatMap((locale) => [`docs_${locale}`, `pages_${locale}`])
      : ["docs", "pages"];

  const urls: SitemapUrl[] = [];

  for (const collection of collections) {
    try {
      const pages = await (
        queryCollection as unknown as (
          event: unknown,
          collection: string,
        ) => { all: () => Promise<Array<Record<string, unknown> & { path?: string }>> }
      )(event, collection).all();

      for (const page of pages) {
        const meta = (page.meta ?? {}) as Record<string, unknown>;
        const pagePath = page.path || "/";

        // Skip pages opted out via `sitemap: false` frontmatter
        if (meta.sitemap === false) continue;

        // Skip .navigation files (navigation configuration, not pages)
        if (isNavigationPath(pagePath)) continue;

        const urlEntry: SitemapUrl = { loc: pagePath };

        if (meta.modifiedAt && typeof meta.modifiedAt === "string") {
          urlEntry.lastmod = meta.modifiedAt.split("T")[0];
        }

        urls.push(urlEntry);
      }
    } catch {
      // Collection might not exist, skip it
    }
  }

  const sitemap = generateSitemap(urls, siteUrl);

  setResponseHeader(event, "content-type", "application/xml");
  return sitemap;
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

function isNavigationPath(path: string): boolean {
  return path.endsWith(".navigation") || path.includes("/.navigation/");
}

// Same env chain docus uses (docus/utils/meta.ts) — absolute locs require
// NUXT_PUBLIC_SITE_URL (or a platform URL var) in the deploy environment.
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

function generateSitemap(urls: SitemapUrl[], siteUrl: string): string {
  const urlEntries = urls
    .map((url) => {
      const loc = siteUrl ? `${siteUrl}${url.loc}` : url.loc;
      let entry = `  <url>\n    <loc>${escapeXml(loc)}</loc>`;

      if (url.lastmod) {
        entry += `\n    <lastmod>${escapeXml(url.lastmod)}</lastmod>`;
      }

      entry += `\n  </url>`;
      return entry;
    })
    .join("\n");

  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urlEntries}
</urlset>`;
}

function escapeXml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}
