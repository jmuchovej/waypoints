import { defineNuxtModule } from "@nuxt/kit";

/**
 * Confines docus's site-wide catch-all page to the docs tree.
 *
 * Docus is built as a docs-only site, so its layer ships
 * `app/pages/[[lang]]/[...slug].vue` matching every path. This site is
 * marketing + docs: marketing pages come from our own catch-all
 * (`app/pages/[...slug].vue`, `pages_*` collections) and only `/docs/**`
 * should reach docus's page (`docs_*` collections).
 *
 * The `[[lang]]` segment is dropped at the same time: @nuxtjs/i18n localizes
 * routes after this hook runs (`pages:resolved`, under Nuxt 4's default
 * `scanPageMeta: "after-resolve"`), so keeping it would double-prefix the
 * locale (`/:locale(en|pt)?/:lang?/…`). Docus never reads `params.lang` —
 * `useDocusI18n` delegates to `$i18n`, which resolves the locale from the
 * i18n-generated prefix.
 */
export default defineNuxtModule({
  meta: { name: "website:routing" },
  setup(_options, nuxt) {
    nuxt.hook("pages:extend", (pages) => {
      const docusPage = pages.find((page) => page.file?.includes("docus/app/pages"));
      if (!docusPage) {
        throw new Error(
          "[website:routing] docus's catch-all page was not found in the pages scan." +
            " The docus layer moved or renamed its pages directory — re-check this" +
            " module against the installed docus version before building.",
        );
      }
      docusPage.name = "docs";
      docusPage.path = "/docs/:slug(.*)*";
    });
  },
});
