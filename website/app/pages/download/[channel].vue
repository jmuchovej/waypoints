<script setup lang="ts">
  import { buildChannelButtons } from "#imports";

  const route = useRoute();
  const channel = computed(() => route.params.channel as UpdateChannel);

  routeGuard(channel);
  const channels = buildChannelButtons("download", channel);

  const date = (date: string) =>
    new Date(date).toLocaleDateString("en-US", {
      year: "numeric",
      month: "long",
      day: "numeric",
    });

  const { data: releases } = await useReleases(channel);

  const latest = computed(() => releases.value?.[0] ?? null);

  const downloadUrl = (platform: Platform, arch: Arch) =>
    (latest.value ? downloadPathFor(channel.value, latest.value.tag, platform, arch) : undefined) ??
    undefined;

  const stores = computed(() => STORE_LINKS[channel.value] ?? []);
</script>

<template>
  <UContainer v-if="latest">
    <UPage>
      <template #left>
        <img :src="`/icons/${channel}.png`" alt="Waypoints logo" class="mx-auto w-full" />
      </template>

      <template #right> </template>

      <UPageHeader
        :title="latest.title"
        headline="Download"
        :description="`Released ${date(latest.date)}`"
        :ui="{
          root: 'border-0',
          container: '!py-0',
          description: 'flex flex-1 items-center justify-between',
        }">
        <template #links>
          <UNavigationMenu :items="channels" variant="pill" />
        </template>
        <template #description>
          <span>Released {{ date(latest.date) }}</span>

          <UButton :to="`/releases/${channel}/${latest.tag}`" variant="ghost">
            See the release notes
          </UButton>
        </template>
      </UPageHeader>

      <UPageBody>
        <UPageGrid class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <UPageCard
            v-for="target in DESKTOP_TARGETS"
            :key="`${target.platform}-${target.arch}`"
            :icon="target.icon"
            :title="target.title"
            :description="target.description"
            :to="downloadUrl(target.platform, target.arch)"
            external />
        </UPageGrid>

        <template v-if="stores.length">
          <USeparator label="On your phone" class="my-10" />

          <UPageGrid class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            <UPageCard
              v-for="store in stores"
              :key="store.to"
              :icon="store.icon"
              :title="store.title"
              :description="store.description"
              :to="store.to"
              target="_blank"
              rel="noopener" />
          </UPageGrid>
        </template>
      </UPageBody>
    </UPage>
  </UContainer>
</template>
