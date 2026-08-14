<script setup lang="ts">
  import type { NavigationMenuItem } from "@nuxt/ui";
  import { toRichText } from "@waypoints/domain";
  import type { Ref } from "vue";

  const sidebarOpen = inject<Ref<boolean>>("sidebar-open", ref(false));
  const newLandmarkOpen = ref(false);

  const tabs: NavigationMenuItem[][] = [
    [
      {
        label: "Recents",
        icon: "i-lucide-clock",
        to: "/library/recents",
      },
      {
        label: "Pinned",
        icon: "i-lucide-pin",
        to: "/library/pinned",
      },
    ],
  ];

  useHead({
    title: "Library",
  });

  const title = computed(() => toRichText("Library"));
</script>

<template>
  <AppHeader :tabs="tabs" :title="title" />

  <div class="flex-1 overflow-auto">
    <NuxtPage />
  </div>
</template>
