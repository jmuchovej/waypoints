<script setup lang="ts">
  const route = useRoute();
  const channel = computed(() => route.params.channel as "stable" | "preview");

  if (!["stable", "preview"].includes(channel.value)) {
    throw createError({ statusCode: 404, statusMessage: "Not Found" });
  }

  const channels = [
    {
      label: "Stable",
      to: "/download/stable",
      active: channel.value === "stable",
    },
    {
      label: "Preview",
      to: "/download/preview",
      active: channel.value === "preview",
    },
  ];

  const date = (date: string | null) =>
    date
      ? new Date(date).toLocaleDateString("en-US", {
          year: "numeric",
          month: "long",
          day: "numeric",
        })
      : null;

  const { data: releases } = await useReleases(channel);

  const latest = computed(() => releases.value?.[0] ?? null);

  const downloadUrl = (target: string) => `/api/v1/releases/${channel.value}/${target}`;

  // Narrowed to the targets with an actual download path — mobile ships
  // through its own app store and is archived without ever being linked
  // here. See the design's "Published formats are the bundler's installers"
  // decision.
  const platforms = [
    {
      icon: "i-simple-icons-apple",
      title: "macOS",
      description: "Apple Silicon",
      target: "macos-arm64",
    },
    {
      icon: "i-simple-icons-linux",
      title: "Linux",
      description: "AppImage (x86_64)",
      target: "linux-amd64",
    },
    {
      icon: "i-simple-icons-windows",
      title: "Windows",
      description: "64-bit installer",
      target: "windows-amd64",
    },
  ];
</script>

<template>
  <main v-if="latest">
    <UContainer>
      <UPage>
        <template #left>
          <img :src="`/icons/${channel}.png`" alt="Waypoints logo" class="mx-auto w-full" />
        </template>

        <template #right> </template>

        <UPageHeader
          :title="latest.title"
          headline="Download"
          :description="date(latest.date) ? `Released ${date(latest.date)}` : undefined"
          :ui="{
            root: 'border-0',
            container: '!py-0',
            description: 'flex flex-1 items-center justify-between',
          }">
          <template #links>
            <UNavigationMenu :items="channels" variant="pill" />
          </template>
          <template #description>
            <span v-if="date(latest.date)">Released {{ date(latest.date) }}</span>

            <UButton :to="`/releases/${channel}/${latest.tag}`" variant="ghost">
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
              :to="downloadUrl(platform.target)"
              target="_blank" />
          </UPageGrid>
        </UPageBody>
      </UPage>
    </UContainer>
  </main>
</template>
