<script setup lang="ts">
  import type { NavigationMenuItem } from "@nuxt/ui";
  import { toRichText } from "@waypoints/domain";

  const tabs: NavigationMenuItem[][] = [
    [
      {
        label: "Profile",
        icon: "i-lucide-user",
        to: "/settings/profile",
      },
      {
        label: "Members",
        icon: "i-lucide-users",
        to: "/settings/members",
      },
      {
        label: "Notifications",
        icon: "i-lucide-bell",
        to: "/settings/notifications",
      },
      {
        label: "Security",
        icon: "i-lucide-shield",
        to: "/settings/security",
      },
      {
        label: "Sync",
        icon: "i-lucide-radio-tower",
        to: "/settings/sync",
      },
    ],
  ];

  useHead({
    title: "Settings",
  });

  const title = computed(() => toRichText("Settings"));
</script>

<template>
  <AppHeader :tabs="tabs" :title="title" />

  <div class="flex-1 overflow-auto">
    <div class="mx-auto flex w-full flex-col gap-4 px-4 py-12 sm:gap-6 lg:max-w-2xl lg:gap-12">
      <NuxtPage />
    </div>
  </div>
</template>
