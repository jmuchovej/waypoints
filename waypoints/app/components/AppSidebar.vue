<script setup lang="ts">
import { ref, inject, type Ref } from "vue";
import type { NavigationMenuItem } from "@nuxt/ui";
import { createReusableTemplate } from "@vueuse/core";
import { LazyUCalendar, UTooltip, UButton } from "#components";

const open = inject<Ref<boolean>>("sidebar-open", ref(false));

const route = useRoute();

const [DefineLibraryLink, ReuseLibraryLink] = createReusableTemplate<{
  name: string;
  icon: string;
}>();

const links: NavigationMenuItem[] = [
  {
    label: "Recents",
    icon: "i-lucide-clock-fading",
    to: "/library/recents",
    slot: "recents" as const,
    type: "label",
    onSelect() {
      open.value = false;
    },
    children: [
      // TODO: dynamically populate with first 5–10 recently visited landmarks

    ],
  },
  {
    label: "Pinned",
    icon: "i-lucide-pin",
    slot: "pinned" as const,
    type: "label",
    to: "/library/pinned",
    defaultOpen: true,
    children: [
      // TODO: dynamically populate with first 5–10 pinned landmarks

    ],
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

const footerLinks: NavigationMenuItem[] = [
  {
    label: "Library",
    icon: "i-lucide-library",
    to: "/library",
    exact: true,
  },
  {
    label: "Trash",
    icon: "i-lucide-trash",
    to: "/trash",
  },
];
</script>

<template>
  <USidebar
    v-model:open="open"
    collapsible="icon"
    variant="inset"
    :ui="{
      container: 'py-0 h-full',
      // inner: 'bg-elevated/25 divide-transparent',
      // body: 'pt-0',
      footer: 'border-t border-default',
    }"
  >
    <template #header="{ state }">
      <TerritoryMenu :collapsed="state" />

      <UButton
        v-if="state === 'expanded'"
        variant="ghost"
        color="neutral"
        size="xs"
        :icon="`i-lucide-chevrons-${open ? 'left' : 'right'}`"
        @click="open = false"
      />
    </template>

    <template #default="{ state }">
      <!-- <LazyUCalendar
        v-model="date"
        :year-controls="false"
        variant="subtle"
        size="xs"
        week-numbers
        fixed-weeks
      /> -->

      <DefineLibraryLink v-slot="{ name, icon }">
        <UTooltip text="Open in Library" :delay-duration="100">
          <UButton
            color="neutral"
            variant="ghost"
            size="xs"
            :icon="icon"
            :to="`/library/${name}`"
          />
        </UTooltip>
      </DefineLibraryLink>

      <UNavigationMenu
        :items="links"
        orientation="vertical"
        :ui="{ link: 'p-1.5 overflow-hidden' }"
        tooltip
        popover
      >
        <template #recents-trailing>
          <ReuseLibraryLink name="recents" icon="i-lucide-library" />
        </template>
        <template #pinned-trailing>
          <ReuseLibraryLink name="pinned" icon="i-lucide-library" />
        </template>
      </UNavigationMenu>

      <UNavigationMenu
        :items="footerLinks"
        orientation="vertical"
        :ui="{ link: 'p-1.5 overflow-hidden' }"
        class="mt-auto"
        tooltip
        popover
      />
    </template>

    <template #footer="{ state }">
      <UserMenu :collapsed="state" />
    </template>
  </USidebar>
</template>
