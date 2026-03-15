<script setup lang="ts">
import { today, getLocalTimeZone } from "@internationalized/date";
import { ref } from "vue";
import type { NavigationMenuItem } from "@nuxt/ui";

const route = useRoute();

const links: NavigationMenuItem[][] = [];
links.push([
  {
    label: "Home",
    icon: "i-lucide-house",
    to: "/",
    onSelect() {
      open.value = false;
    },
  },
  {
    label: "Settings",
    icon: "i-lucide-settings",
    to: "/settings",
    defaultOpen: true,
    type: "trigger",
    children: [
      {
        label: "General",
        to: "/settings",
        exact: true,
        onSelect() {
          open.value = false;
        },
      },
      {
        label: "Members",
        to: "/settings/members",
        onSelect() {
          open.value = false;
        },
      },
      {
        label: "Notifications",
        to: "/settings/notifications",
        onSelect() {
          open.value = false;
        },
      },
      {
        label: "Security",
        to: "/settings/security",
        onSelect() {
          open.value = false;
        },
      },
    ],
  },
]);

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
            :collapsed="collapsed"
            :items="links[0]"
            orientation="vertical"
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
