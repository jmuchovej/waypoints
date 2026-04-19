<script setup lang="ts">
import type { Chart, ChartID, LandmarkMark } from "@waypoints/domain";
import { ipc, iconName, CommandMenuKey, type CommandMenuContext } from "#imports";

const FALLBACK_ICON = "i-lucide-file-text";

const overlay = inject<CommandMenuContext>(CommandMenuKey);

const landmarks = ref<LandmarkMark[]>([]);
const charts = ref<Map<ChartID, Chart>>(new Map());
const selectedIndex = ref(0);

async function loadLandmarks() {
  try {
    const [markData, chartData] = await Promise.all([
      ipc.marks.list("landmark"),
      ipc.charts.list(),
    ]);
    landmarks.value = markData;
    charts.value = new Map(chartData.map((c) => [c.id, c]));
  } catch {
    landmarks.value = [];
  }
}

function landmarkIcon(mark: LandmarkMark): string {
  const own = iconName(mark.data.icon);
  if (own) return own;

  const chart = charts.value.get(mark.data.chart);
  const chartIcon = iconName(chart?.icon);
  if (chartIcon) return chartIcon;

  return FALLBACK_ICON;
}

onMounted(loadLandmarks);

const filteredItems = computed(() => {
  const query = overlay?.activeMenu.value?.query?.toLowerCase() ?? "";
  if (!query) return landmarks.value;
  return landmarks.value.filter((m) => {
    const title = m.data.title.plain_text?.toLowerCase() ?? "";
    return title.includes(query);
  });
});

watch(
  () => overlay?.activeMenu.value?.query,
  () => {
    selectedIndex.value = 0;
  },
);

function selectItem(landmark: LandmarkMark) {
  if (!overlay?.activeMenu.value) return;

  const title = landmark.data.title.plain_text || "Untitled";
  const onSelect = overlay.activeMenu.value.onSelect;

  if (onSelect) {
    // Build a ProseMirror text node with the mention mark
    onSelect({
      type: "text",
      text: title,
      marks: [{ type: "mention", attrs: { id: landmark.id } }],
    });
  }

  overlay.hide();
}

function handleKeydown(e: KeyboardEvent) {
  if (!overlay?.isOpen.value) return;

  const items = filteredItems.value;
  if (items.length === 0 && e.key !== "Escape") return;

  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      e.stopPropagation();
      selectedIndex.value = (selectedIndex.value + 1) % items.length;
      break;
    case "ArrowUp":
      e.preventDefault();
      e.stopPropagation();
      selectedIndex.value = (selectedIndex.value - 1 + items.length) % items.length;
      break;
    case "Enter":
    case "Tab":
      e.preventDefault();
      e.stopPropagation();
      if (items[selectedIndex.value]) {
        selectItem(items[selectedIndex.value]!);
      }
      break;
    case "Escape":
      e.preventDefault();
      e.stopPropagation();
      overlay.hide();
      break;
  }
}

defineExpose({ handleKeydown });
</script>

<template>
  <Teleport to="body">
    <div
      v-if="overlay?.isOpen.value && overlay.activeMenu.value?.kind === 'mention'"
      class="fixed z-50 bg-elevated border border-default rounded-lg shadow-lg py-1 w-64 max-h-60 overflow-y-auto"
      :style="{
        left: `${overlay.activeMenu.value.position.x}px`,
        top: `${overlay.activeMenu.value.position.y}px`,
      }"
    >
      <div v-if="filteredItems.length === 0" class="px-3 py-2 text-sm text-dimmed italic">
        No landmarks found
      </div>
      <button
        v-for="(item, i) in filteredItems"
        :key="item.id"
        class="w-full flex items-center gap-2 px-3 py-1.5 text-sm text-default cursor-pointer transition-colors"
        :class="{ 'bg-accented': i === selectedIndex }"
        @click="selectItem(item)"
        @mouseenter="selectedIndex = i"
      >
        <UIcon :name="landmarkIcon(item)" class="size-4 shrink-0 text-dimmed" />
        <span class="truncate">
          {{ item.data.title.plain_text || "Untitled" }}
        </span>
      </button>
    </div>
  </Teleport>
</template>
