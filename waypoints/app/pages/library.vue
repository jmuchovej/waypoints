<script setup lang="ts">
import type { NavigationMenuItem } from "@nuxt/ui";
import type { Ref } from "vue";
import { toRichText } from "@waypoints/domain";

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

  <div class="overflow-auto flex-1">
    <NuxtPage />
  </div>
</template>
