<!--
  SPDX-FileCopyrightText: 2026 John Muchovej
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script setup lang="ts">
  import type { NavigationMenuItem } from "@nuxt/ui";
  import { toRichText } from "@waypoints/domain";
  import type { Ref } from "vue";

  const sidebarOpen = inject<Ref<boolean>>("sidebar-open", ref(false));
  const newLandmarkOpen = ref(false);

  // On a phone `sidebar-open` drives the navigation menu, so `?sidebar=open`
  // shows what the bare route cannot. Applied once mounted: USidebar closes
  // itself when it first measures a phone-width viewport, which would undo a
  // pose set any earlier (a link that launched the app lands before then).
  const pose = usePose({ sidebar: ["open", "closed"] });
  onMounted(async () => {
    await nextTick();
    watch(
      () => pose.value.sidebar,
      (sidebar) => {
        if (sidebar) sidebarOpen.value = sidebar === "open";
      },
      { immediate: true },
    );
  });

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
