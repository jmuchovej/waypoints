<script setup lang="ts">
  import type { ThemeUI } from "@nuxt/ui";

  import { buildChannelButtons } from "#imports";
  import type { Channel } from "#shared/utils/releases";

  const route = useRoute();
  const channel = computed(() => route.params.channel as Channel);
  const version = computed(() => route.params.version as string | undefined);

  routeGuard(channel);
  const channels = buildChannelButtons("releases", channel);

  const { data: releases } = await useReleases(channel);

  const filtered = computed(() => {
    if (!releases.value) return [];
    if (version.value) {
      return releases.value.filter((r) => r.tag === version.value);
    }
    return releases.value;
  });

  if (version.value && !filtered.value.length) {
    throw createError({ statusCode: 404, statusMessage: "Release not found" });
  }

  const theming: ThemeUI = {
    contentToc: {
      root: "w-full",
    },
    page: {
      root: "md:border-x border-default p-4 lg:p-12",
    },
    pageHero: {
      root: "md:border-b border-default",
      container: "relative py-10 sm:py-16 lg:py-24",
    },
    changelogVersion: {
      root: "flex items-start",
      container: "w-full max-w-xl",
      header: "border-b border-default pb-4",
      title: "text-3xl",
      date: "text-xs/9 text-highlighted font-mono",
      indicator: "sticky top-0 pt-16 -mt-16 sm:pt-24 sm:-mt-24 lg:pt-32 lg:-mt-32",
      // root: 'flex',
      // container: 'ms-0 min-w-0',
      // header: 'border-b border-default pb-4',
      // title: 'text-2xl sm:text-3xl',
      // date: 'text-xs text-muted font-mono',
      // indicator:
      // 'justify-start sticky top-(--ui-header-height) pt-4 -mt-4 flex flex-col items-end',
    },
  };
</script>

<template>
  <UTheme :ui="theming">
    <UPageHero title="Releases" :links="channels">
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
        <UPageBody class="max-w-5xl">
          <UChangelogVersions :indicator="false" v-if="filtered.length">
            <UChangelogVersion
              v-for="release in filtered"
              :key="release.tag"
              :title="release.title"
              :date="release.date ?? undefined"
              :to="version ? undefined : `/releases/${channel}/${release.tag}`"
              :indicator="false">
              <template #body>
                <MDC v-if="release.markdown" :value="release.markdown" />
                <p v-else class="text-muted italic">Release notes are unavailable.</p>
              </template>
            </UChangelogVersion>
          </UChangelogVersions>

          <div v-else class="text-muted py-24 text-center">
            <p>No {{ channel }} releases yet.</p>
          </div>
        </UPageBody>
      </UPage>
    </UContainer>
  </UTheme>
</template>
