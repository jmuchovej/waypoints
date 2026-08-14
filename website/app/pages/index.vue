<script setup lang="ts">
  import type { PageCollections } from "@nuxt/content";
  import type { ThemeUI } from "@nuxt/ui";

  // Narrow to the page collections — casting to `keyof Collections` would pull in
  // the roadmap *data* collection, which has no title/seo/body.
  type PagesKey = keyof PageCollections & (`pages_${string}` | "pages");

  const route = useRoute();
  const { locale, isEnabled } = useDocusI18n();

  // Dynamic collection name based on i18n status
  const collectionName = computed(
    () => (isEnabled.value ? `pages_${locale.value}` : "pages") as PagesKey,
  );

  const { data: page } = await useAsyncData(collectionName.value, () =>
    queryCollection(collectionName.value).path(route.path).first(),
  );
  if (!page.value) {
    throw createError({
      statusCode: 404,
      statusMessage: "Page not found",
      fatal: true,
    });
  }

  const title = page.value.seo?.title || page.value.title;
  const description = page.value.seo?.description || page.value.description;

  useSeoMeta({
    title,
    description,
    ogType: "website",
    ogTitle: page.value.seo?.title,
    ogDescription: page.value.seo?.description,
    ogImage: page.value?.seo?.ogImage as string | undefined,
  });
  useHead({
    title,
    titleTemplate: "%s",
  });

  if (!page.value?.seo?.ogImage) {
    defineOgImage("Landing", {
      title: title?.slice(0, 60),
      description: formatOgDescription(title, description),
    });
  }

  const theming: ThemeUI = {
    page: {
      root: "md:border-x border-default",
    },
    pageHero: {
      // root: "md:border-b border-default",
      container: "relative py-10 sm:py-16 lg:py-24",
    },
    pageBody: {},
    pageSection: {
      root: "divide-inherit",
    },
    prose: {
      p: {
        base: "my-0",
      },
    },
  };
</script>

<template>
  <UTheme :ui="theming" v-if="page">
    <UPage>
      <ContentRenderer class="divide-default divide-y" :value="page" />
    </UPage>
  </UTheme>
</template>
