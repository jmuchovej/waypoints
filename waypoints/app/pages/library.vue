<script setup lang="ts">
import type { NavigationMenuItem } from "@nuxt/ui";
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
</script>

<template>
  <AppHeader :tabs="tabs" title="Library" />

  <div class="overflow-auto flex-1">
    <NuxtPage />
  </div>

  <NewLandmarkPalette v-model:open="newLandmarkOpen" />
</template>
