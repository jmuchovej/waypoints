<script setup lang="ts">
import { today, getLocalTimeZone } from "@internationalized/date";
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { AppSidebar } from "#components";

const route = useRoute();

const siderbarOpen = ref(true);
provide("sidebar-open", siderbarOpen);

const newLandmarkOpen = ref(false);
const date = shallowRef(today(getLocalTimeZone()));

// Listen for File > New menu event from native menu (⌘N / Ctrl+N)
let unlisten: (() => void) | undefined;
onMounted(async () => {
  unlisten = await listen("app:mark:new", () => {
    newLandmarkOpen.value = true;
  });
});
onUnmounted(() => unlisten?.());

useHead({
  titleTemplate: (title) => title ?? "Waypoints",
  charset: "utf-8",
  viewport: "width=device-width, initial-scale=1",
  meta: [
    { name: "format-detection", content: "no" },
    {
      name: "viewport",
      content: [
        "width=device-width",
        "initial-scale=1",
        "maximum-scale=1",
        "viewport-fit=cover",
      ].join(", "),
    },
  ],
});

import { getCurrentWindow } from "@tauri-apps/api/window";
const appWindow = getCurrentWindow();

watch(
  () => route.fullPath,
  async () => {
    await nextTick(); // let useHead flush the new document.title first
    await appWindow.setTitle(document.title || "Waypoints");
  },
  { immediate: true },
);
</script>

<template>
  <div class="flex flex-row h-dvh">
    <AppSidebar />

    <NewLandmarkPalette v-model:open="newLandmarkOpen" />

    <div class="page-content">
      <slot />
    </div>
  </div>
</template>
