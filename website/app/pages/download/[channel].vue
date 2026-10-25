<script setup lang="ts">
  import { buildChannelButtons } from "#imports";
  import {
    type Channel,
    DOWNLOADABLE_TARGETS,
    latestUrl,
    targetSlug,
  } from "#shared/utils/releases";

  const route = useRoute();
  const channel = computed(() => route.params.channel as Channel);

  routeGuard(channel);
  const channels = buildChannelButtons("download", channel);

  const date = (date: string | null) =>
    date
      ? new Date(date).toLocaleDateString("en-US", {
          year: "numeric",
          month: "long",
          day: "numeric",
        })
      : null;

  // Client-side only, deliberately — the links below never depend on this,
  // so a slow or failed fetch degrades the heading, never the download
  // surface. See the design's "The download page is static; only the
  // displayed version is live" decision.
  const { data: current } = useCurrentRelease(channel);

  // Narrowed to the targets with an actual download path — mobile ships
  // through its own app store and is archived without ever being linked
  // here. See the design's "Published formats are the bundler's installers"
  // decision. Links point at the versionless alias, never a resolved
  // version — see the design's "`latest` is a redirect to the versioned
  // URL" decision, which is what keeps this page's markup unchanged by a
  // publish.
  const PLATFORM_META: Record<string, { icon: string; title: string; description: string }> = {
    macos: { icon: "i-simple-icons-apple", title: "macOS", description: "Apple Silicon" },
    linux: { icon: "i-simple-icons-linux", title: "Linux", description: "AppImage (x86_64)" },
    windows: { icon: "i-simple-icons-windows", title: "Windows", description: "64-bit installer" },
  };
  const platforms = DOWNLOADABLE_TARGETS.map((target) => ({
    ...PLATFORM_META[target.os],
    target: targetSlug(target),
    url: latestUrl(channel.value, target.os, target.arch, target.extensions[0]!),
  }));

  const stores = computed(() => STORE_LINKS[channel.value] ?? []);
</script>

<template>
  <UContainer>
    <UPage>
      <template #left>
        <img :src="`/icons/${channel}.png`" alt="Waypoints logo" class="mx-auto w-full" />
      </template>

      <template #right> </template>

      <UPageHeader
        :title="current?.title ?? 'Waypoints'"
        headline="Download"
        :description="date(current?.date ?? null) ? `Released ${date(current!.date)}` : undefined"
        :ui="{
          root: 'border-0',
          container: '!py-0',
          description: 'flex flex-1 items-center justify-between',
        }">
        <template #links>
          <UNavigationMenu :items="channels" variant="pill" />
        </template>
        <template #description>
          <span v-if="date(current?.date ?? null)">Released {{ date(current!.date) }}</span>

          <UButton v-if="current" :to="`/releases/${channel}/${current.tag}`" variant="ghost">
            See the release notes
          </UButton>
        </template>
      </UPageHeader>

      <UPageBody>
        <UPageGrid class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <UPageCard
            v-for="platform in platforms"
            :key="platform.target"
            :icon="platform.icon"
            :title="platform.title"
            :description="platform.description"
            :to="platform.url"
            target="_blank" />
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
