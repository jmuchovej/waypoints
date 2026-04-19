<script setup lang="ts">
import { LazyUAvatar, LazyUCommandPalette } from "#components";
import type { Chart } from "@waypoints/domain";
import { iconName, ipc } from "#imports";

const open = defineModel<boolean>("open", { default: false });

const log = createLogger("new-landmark");

const charts = ref<Chart[]>([]);

async function loadCharts() {
  try {
    charts.value = await ipc.charts.list();
  } catch (e) {
    log.error("chart_list failed:", e);
  }
}

onMounted(loadCharts);

// Param and return are `any` to keep TS from instantiating `Chart` against
// Nuxt UI's `CommandPaletteItem` type at the `.map` site — its recursive
// Iconify icon union otherwise trips TS2589 (excessively deep instantiation,
// and budget-sensitive/flaky). The shape is exercised by the template.
function toPaletteItem(chart: Chart): any {
  return {
    label: chart.name,
    icon: iconName(chart.icon, "i-lucide-file-text"),
    suffix: chart.description ?? undefined,
    onSelect: () => createLandmark(chart),
  };
}

const groups = computed<any[]>(() => [
  {
    id: "charts",
    label: "Create New",
    items: (charts.value as any[]).map(toPaletteItem),
  },
]);

async function createLandmark(chart: Chart) {
  open.value = false;
  try {
    const mark = await ipc.charts.createLandmark(chart.id);
    await navigateTo(`/marks/${mark.id}`);
  } catch (e) {
    log.error("landmark_create failed:", e);
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
