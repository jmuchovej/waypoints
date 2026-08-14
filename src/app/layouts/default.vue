<script setup lang="ts">
  import { today, getLocalTimeZone } from "@internationalized/date";
  import { ref } from "vue";

  import { AppSidebar } from "#components";
  import { ipc, listen } from "#imports";

  const route = useRoute();

  const siderbarOpen = ref(true);
  provide("sidebar-open", siderbarOpen);

  const newLandmarkOpen = ref(false);
  provide("new-landmark-open", newLandmarkOpen);
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
    meta: [
      { charset: "utf-8" },
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

  watch(
    () => route.fullPath,
    async () => {
      await nextTick(); // let useHead flush the new document.title first
      await setNativeTitle(document.title || "Waypoints");
    },
    { immediate: true },
  );
</script>

<template>
  <div class="flex h-dvh flex-row">
    <AppSidebar />

    <NewLandmarkPalette v-model:open="newLandmarkOpen" />

    <div class="page-content">
      <slot />
    </div>
  </div>
</template>
