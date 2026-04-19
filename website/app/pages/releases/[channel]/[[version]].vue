<script setup lang="ts">
const route = useRoute();
const channel = computed(() => route.params.channel as "stable" | "preview");
const version = computed(() => route.params.version as string | undefined);

if (!["stable", "preview"].includes(channel.value)) {
  throw createError({ statusCode: 404, statusMessage: "Not Found" });
}

const channels = [
  {
    label: "Stable",
    to: "/releases/stable",
    active: channel.value === "stable",
  },
  {
    label: "Preview",
    to: "/releases/preview",
    active: channel.value === "preview",
  },
];

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
</script>

<template>
  <div class="max-w-3xl mx-auto px-4 sm:px-6 py-16 sm:py-24">
    <div class="flex items-center justify-between mb-12">
      <h1 class="text-3xl sm:text-4xl font-bold text-highlighted">Releases</h1>

      <UNavigationMenu
        :items="channels"
        variant="pill"
        :ui="{
          // root: 'bg-muted rounded-lg px-1',
        }"
      />
    </div>

    <UChangelogVersions :indicator-motion="false" v-if="filtered.length">
      <UChangelogVersion
        v-for="release in filtered"
        :key="release.tag"
        :title="release.title"
        :date="release.date"
        :to="version ? undefined : `/releases/${channel}/${release.tag}`"
        :ui="{
          root: 'flex',
          container: 'ms-0 min-w-0',
          // header: 'border-b border-default pb-4',
          // title: 'text-2xl sm:text-3xl',
          // date: 'text-xs text-muted font-mono',
          indicator:
            'justify-start sticky top-(--ui-header-height) pt-4 -mt-4 flex flex-col items-end',
        }"
      >
        <template #body>
          <MDC :value="release.markdown" />
        </template>
      </UChangelogVersion>
    </UChangelogVersions>

    <div v-else class="text-center py-24 text-muted">
      <p>No {{ channel }} releases yet.</p>
    </div>
  </div>
</template>
