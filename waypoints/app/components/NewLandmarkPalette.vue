<script setup lang="ts">
import { LazyUAvatar, LazyUCommandPalette } from "#components";
import type { CommandPaletteItem } from "@nuxt/ui";
import { commands } from "~/bindings.g";
import type { Chart } from "~/bindings.g";
import { iconName } from "~/utils/icon";

const open = defineModel<boolean>("open", { default: false });

const log = createLogger("new-landmark");

const charts = ref<Chart[]>([]);

async function loadCharts() {
  const result = await commands.chartList();
  if (result.status === "ok") {
    charts.value = result.data;
  } else {
    log.error("chart_list failed:", result.error);
  }
}

onMounted(loadCharts);

const groups = computed(() => [
  {
    id: "charts",
    label: "Create New",
    items: charts.value.map(
      (chart): CommandPaletteItem => ({
        label: chart.name,
        icon: iconName(chart.icon, "i-lucide-file-text"),
        suffix: chart.description ?? undefined,
        onSelect: () => createLandmark(chart),
      }),
    ),
  },
]);

async function createLandmark(chart: Chart) {
  open.value = false;
  const result = await commands.landmarkCreate(chart.id);
  if (result.status === "ok") {
    await navigateTo(`/marks/${result.data.id}`);
  } else {
    log.error("landmark_create failed:", result.error);
  }
}
</script>

<template>
  <UModal v-model:open="open" :ui="{ content: 'p-0' }">
    <template #content>
      <LazyUCommandPalette
        :groups="groups"
        placeholder="Pick a type..."
        class="h-80"
        :ui="{
          item: 'flex flex-row items-center gap-3',
          itemLabel: 'flex flex-col flex-1',
        }"
      >
        <template #item-leading="{ item, ui }">
          <LazyUAvatar
            :icon="item.icon"
            data-slot="itemLeadingIcon"
            :class="{ active: item.active }"
            :ui="{ root: 'rounded' }"
          />
        </template>
      </LazyUCommandPalette>
    </template>
  </UModal>
</template>
