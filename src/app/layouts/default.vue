<script setup lang="ts">
import { today, getLocalTimeZone } from "@internationalized/date";
import { ref } from "vue";
import type { NavigationMenuItem } from "@nuxt/ui";

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
      >
        <template #default="{ collapsed }">
          <TerritoryMenu :collapsed="collapsed" />

          <UDashboardSearchButton :collapsed="collapsed" class="bg-transparent ring-default" />

          <UCalendar
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
      </UDashboardSidebar>

      <UDashboardSearch :groups="[]" />

      <slot />
    </UDashboardGroup>
  </div>
</template>
