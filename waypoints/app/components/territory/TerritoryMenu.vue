<script setup lang="ts">
import type { DropdownMenuItem } from "@nuxt/ui";
import type { Territory } from "@waypoints/cartograph";
import { UDropdownMenu, UButton, LazyTerritoryPicker } from "#components";
import { ipc, createLogger } from "#imports";

const logger = createLogger("TerritoryMenu");

type SidebarState = "expanded" | "collapsed";

const props = defineProps<{
  collapsed?: SidebarState;
}>();

const active = ref<Territory | null>(null);
const territories = ref<Territory[]>([]);
const modalOpen = ref(false);

async function load() {
  logger.debug("Attempting to load territories");
  try {
    active.value = (await ipc.territory.getActive()) ?? null;
    territories.value = await ipc.territory.list();
  } catch (e) {
    logger.error("IPC failed during territory load", e);
  }
}

async function switchTerritory(id: string) {
  try {
    await ipc.territory.open(id);
    reloadNuxtApp({ path: "/" });
  } catch (e) {
    logger.error("Failed to switch territory", e);
  }
}

function onCreated() {
  reloadNuxtApp({ path: "/" });
}

onMounted(load);

const items = computed<DropdownMenuItem[][]>(() => [
  territories.value.map((t) => ({
    label: t.name,
    icon: t.kind === "managed" ? "i-lucide-map" : "i-lucide-folder",
    type: "checkbox" as const,
    checked: active.value?.id === t.id,
    onSelect(e: Event) {
      if (t.id !== active.value?.id) {
        switchTerritory(t.id);
      }
      e.preventDefault();
    },
  })),
  [
    {
      label: "Add Territory",
      icon: "i-lucide-plus",
      onSelect() {
        modalOpen.value = true;
      },
    },
  ],
]);

const collapsed = computed<boolean>(() => props.collapsed === "collapsed");
</script>

<template>
  <UDropdownMenu
    :items="items"
    :content="{ align: 'center', collisionPadding: 12 }"
    :ui="{
      content: collapsed ? 'min-w-48' : 'w-(--reka-dropdown-menu-trigger-width)',
    }"
  >
    <UButton
      :label="collapsed ? undefined : (active?.name ?? 'Waypoints')"
      icon="i-lucide-map"
      :trailing-icon="collapsed ? undefined : 'i-lucide-chevrons-up-down'"
      color="neutral"
      variant="ghost"
      block
      :square="collapsed"
      class="data-[state=open]:bg-elevated"
      :ui="{ trailingIcon: 'text-dimmed' }"
    />
  </UDropdownMenu>

  <UModal
    v-model:open="modalOpen"
    title="Add Territory"
    description="Create a new territory or import an existing one."
    :ui="{ title: 'sr-only', description: 'sr-only' }"
  >
    <template #content>
      <div class="relative p-6 flex justify-center">
        <UButton
          icon="i-lucide-x"
          variant="ghost"
          color="neutral"
          size="sm"
          square
          class="absolute top-2 right-2"
          @click="modalOpen = false"
        />
        <LazyTerritoryPicker @success="onCreated" />
      </div>
    </template>
  </UModal>
</template>
