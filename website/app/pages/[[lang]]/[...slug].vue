<script setup lang="ts">
  import type { ContentNavigationItem, Collections, DocsCollectionItem } from "@nuxt/content";
  import { findPageHeadline } from "@nuxt/content/utils";
  import { kebabCase } from "scule";

  definePageMeta({
    layout: "default",
  });

  const route = useRoute();
  const { locale, isEnabled, t } = useDocusI18n();
  const appConfig = useAppConfig();
  const navigation = inject<Ref<ContentNavigationItem[]>>("navigation");
  const { shouldPushContent: shouldHideToc } = useAssistant();

  const collectionName = "pages";

  const [{ data: page }, { data: surround }] = await Promise.all([
    useAsyncData(
      kebabCase(route.path),
      () => queryCollection(collectionName).path(route.path).first() as Promise<DocsCollectionItem>,
    ),
    useAsyncData(`${kebabCase(route.path)}-surround`, () => {
      return queryCollectionItemSurroundings(collectionName, route.path, {
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

  useSeo({
    title,
    description,
    type: "article",
    modifiedAt: (page.value as unknown as Record<string, unknown>).modifiedAt as string | undefined,
    breadcrumbs,
  });
  watch(
    () => navigation?.value,
    () => {
      headline.value = findPageHeadline(navigation?.value, page.value?.path) || headline.value;
    },
  );

  defineOgImage("Docs", {
    headline: headline.value,
    title: title?.slice(0, 60),
    description: formatOgDescription(title, description),
  });

  // Add the page path to the prerender list
  addPrerenderPath(`/raw${route.path}.md`);
</script>

<template>
  <UPage v-if="page" :key="`page-${shouldHideToc}`">
    <UPageBody>
      <ContentRenderer v-if="page" :value="page" />
    </UPageBody>
  </UPage>
</template>
