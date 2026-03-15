<script setup lang="ts">
import { today, getLocalTimeZone } from "@internationalized/date";
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { NavigationMenuItem } from "@nuxt/ui";
import {
  LazyUCalendar,
  UDashboardGroup,
  UDashboardSearch,
  UDashboardSearchButton,
  UDashboardSidebar,
} from "#components";

const route = useRoute();

const links = (state: "collapsed" | "expanded"): NavigationMenuItem[] => {
  const base = [
    {
      label: "Home",
      icon: "i-lucide-house",
      to: "/",
      onSelect() {
        open.value = false;
      },
    },
    {
      label: "Marks",
      icon: "i-lucide-map-pin",
      to: "/marks",
      onSelect() {
        open.value = false;
      },
    },
  ];

  const pins = {
    label: "Pinned",
    icon: "i-lucide-pin",
    onSelect() {
      open.value = false;
    },
    defaultOpen: true,
    children: state === "expanded" ? [] : [],
  } satisfies NavigationMenuItem;

  const recents = {
    label: "Recent",
    icon: "i-lucide-clock-fading",
    onSelect() {
      open.value = false;
    },
    defaultOpen: true,
    children: state === "expanded" ? [] : [],
  } satisfies NavigationMenuItem;

  return [...base, pins, recents];
};

const open = ref(false);
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
  title: "Waypoints",
  charset: "utf-8",
  viewport: "width=device-width, initial-scale=1",
  meta: [
    { name: "format-detection", content: "no" },
    {
      name: "viewport",
      content: "width=device-width, initial-scale=1, maximum-scale=1, viewport-fit=cover",
    },
  ],
});
</script>

<template>
  <div class="flex flex-col h-dvh">
    <!-- <Titlebar /> -->
    <UDashboardGroup unit="rem">
      <UDashboardSidebar
        id="default"
        v-model:open="open"
        resizable
        collapsible
        class="bg-elevated/25"
        :ui="{
          footer: 'border-t border-default',
        }"
      >
        <template #default="{ collapsed }">
          <TerritoryMenu :collapsed="collapsed" />

          <UDashboardSearchButton :collapsed="collapsed" class="bg-transparent ring-default" />
          <LazyUCalendar
            v-model="date"
            :year-controls="false"
            variant="subtle"
            size="xs"
            week-numbers
            fixed-weeks
          />

          <UNavigationMenu
            :key="collapsed ? 'collapsed' : 'expanded'"
            :items="links(collapsed ? 'collapsed' : 'expanded')"
            orientation="vertical"
            :ui="{ link: 'p-1.5 overflow-hidden' }"
            tooltip
            popover
          />
        </template>

        <template #footer="{ collapsed }">
          <UserMenu :collapsed="collapsed" />
        </template>
      </UDashboardSidebar>

      <UDashboardSearch :groups="[]" />
      <NewLandmarkPalette v-model:open="newLandmarkOpen" />

      <slot />
    </UDashboardGroup>
  </div>
</template>
