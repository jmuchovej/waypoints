<script setup lang="ts">
  import type { ContentNavigationItem, DocsCollectionItem, PageCollections } from "@nuxt/content";
  import { findPageHeadline } from "@nuxt/content/utils";
  import type { ThemeUI } from "@nuxt/ui";
  import { kebabCase } from "scule";

  definePageMeta({
    layout: "default",
  });

  type PagesKey = keyof PageCollections & (`pages_${string}` | "pages");

  const route = useRoute();
  const { locale, isEnabled, t } = useDocusI18n();
  const navigation = inject<Ref<ContentNavigationItem[]>>("navigation");

  const collectionName = computed(
    () => (isEnabled.value ? `pages_${locale.value}` : "pages") as PagesKey,
  );

  const [{ data: page }, { data: surround }] = await Promise.all([
    useAsyncData(kebabCase(route.path), () =>
      queryCollection(collectionName.value).path(route.path).first(),
    ),
    useAsyncData(`${kebabCase(route.path)}-surround`, () => {
      return queryCollectionItemSurroundings(collectionName.value, route.path, {
        fields: ["description"],
      });
    }),
  ]);

  if (!page.value) {
    throw createError({
      statusCode: 404,
      statusMessage: "Page not found",
      fatal: true,
    });
  }

  const title = page.value.seo?.title || page.value.title;
  const description = page.value.seo?.description || page.value.description;

  const headline = ref(findPageHeadline(navigation?.value, page.value?.path));
  const breadcrumbs = computed(() =>
    findPageBreadcrumbs(navigation?.value, page.value?.path || ""),
  );

  useHead({ title });
  useSeo({
    title,
    description,
    type: "article",
    modifiedAt: (page.value as unknown as Record<string, unknown>).modifiedAt as string | undefined,
    breadcrumbs,
  });
  useSchemaOrg([
    defineArticle({
      headline: () => headline.value,
      description: () => description ?? "",
      datePublished: () => "",
      dateModified: () => "",
    }),
  ]);
  watch(
    () => navigation?.value,
    () => {
      headline.value = findPageHeadline(navigation?.value, page.value?.path) || headline.value;
    },
  );

  defineOgImage("Pages", {
    headline: headline.value,
    title: title?.slice(0, 60),
    description: formatOgDescription(title, description),
  });

  console.log(page.value?.body?.toc?.links);

  // Add the page path to the prerender list
  addPrerenderPath(`/raw${route.path}.md`);

  const theming: ThemeUI = {
    contentToc: {
      root: "w-full",
    },
    page: {
      root: "md:border-x border-default p-4 lg:p-12",
      center: "lg:col-span-7",
      right: "lg:col-span-3",
    },
    pageHero: {
      root: "md:border-b border-default",
      container: "relative py-10 sm:py-16 lg:py-24",
    },
  };
</script>

<template>
  <UTheme :ui="theming" v-if="page">
    <UPageHero :title="page.title" :description="page.description">
      <template #top>
        <div
          class="bg-primary absolute left-1/2 z-[-1] size-60 -translate-x-1/2 -translate-y-80 transform rounded-full blur-[300px] sm:size-80" />
      </template>

      <BackgroundStars />

      <div
        aria-hidden="true"
        class="border-default absolute inset-0 z-[-1] mx-4 hidden border-x sm:mx-6 md:block lg:mx-8" />
    </UPageHero>

    <UContainer>
      <UPage>
        <UPageBody>
          <ContentRenderer :value="page" />
        </UPageBody>

        <template #right>
          <!-- DocsAsideRight only reads page.body.toc; its prop is typed
               DocsCollectionItem because Docus wrote it for the docs tree. -->
          <DocsAsideRight :page="page as unknown as DocsCollectionItem" />
        </template>
      </UPage>
    </UContainer>
  </UTheme>
</template>
