<script setup lang="ts">
import type { RichText, Chart, ChartID, LandmarkMark } from "@waypoints/domain";
import { toRichText } from "@waypoints/domain";
import { RichTextEditor } from "#components";
import { navigateTo, ipc, iconName } from "#imports";

const log = createLogger("marks");

const landmarks = ref<LandmarkMark[]>([]);
const charts = ref<Map<ChartID, Chart>>(new Map());
const loading = ref(true);
const newLandmarkOpen = ref(false);

/** Fallback icon when neither the landmark nor its chart has one set. */
const FALLBACK_ICON = "i-lucide-file-text";

async function load() {
  loading.value = true;

  try {
    const [markData, chartData] = await Promise.all([
      ipc.marks.list("landmark"),
      ipc.charts.list(),
    ]);
    landmarks.value = markData;
    charts.value = new Map(chartData.map((c) => [c.id, c]));
  } catch (e) {
    log.warn("failed to load marks/charts:", e);
  }

  loading.value = false;
}

onMounted(load);

function landmarkTitle(mark: LandmarkMark): RichText {
  return mark.data.title;
}

function landmarkIcon(mark: LandmarkMark): string {
  const own = iconName(mark.data.icon);
  if (own) return own;

  const chart = charts.value.get(mark.data.chart);
  const chartIcon = iconName(chart?.icon);
  if (chartIcon) return chartIcon;

  return FALLBACK_ICON;
}

async function onTitleUpdate(mark: LandmarkMark, richText: RichText) {
  mark.data.title = richText;
  try {
    await ipc.editor.updateTitle(mark.id, richText);
  } catch (e) {
    log.warn("update title failed:", e);
  }
}

useHead({
  title: "Marks",
});

const title = computed(() => toRichText("Marks"));
</script>

<template>
  <AppHeader :tabs="[]" :title="title" />

  <div class="overflow-auto flex-1">
    <div v-if="loading" class="flex items-center justify-center h-full">
      <UIcon name="i-lucide-loader" class="animate-spin size-5 text-dimmed" />
    </div>

    <div
      v-else-if="landmarks.length === 0"
      class="flex flex-col items-center justify-center h-full gap-2 text-dimmed"
    >
      <UIcon name="i-lucide-map-pin-off" class="size-8" />
      <p>No landmarks yet</p>
    </div>

    <nav v-else class="flex flex-col p-2 gap-0.5">
      <div
        v-for="mark in landmarks"
        :key="mark.id"
        role="link"
        tabindex="0"
        class="flex items-center gap-2 ps-3 pe-1 py-1 rounded-md hover:bg-elevated transition-colors cursor-pointer focus:outline focus:outline-2 focus:outline-primary"
        @click="navigateTo(`/marks/${mark.id}`)"
        @keydown.enter="navigateTo(`/marks/${mark.id}`)"
      >
        <span class="flex items-center shrink-0" @click.stop>
          <UIcon :name="landmarkIcon(mark)" class="size-5 text-dimmed" />
        </span>
        <RichTextEditor
          :rich-text="landmarkTitle(mark)!"
          :on-update="(rt: RichText) => onTitleUpdate(mark, rt)"
          placeholder="Untitled"
          class="min-w-0 shrink truncate"
          @click.stop
        />
        <UButton
          :to="`/marks/${mark.id}`"
          leading-icon="i-lucide-fullscreen"
          label="Open"
          variant="soft"
          color="neutral"
          size="sm"
          class="ms-auto shrink-0"
          @click.stop
        />
      </div>
    </nav>
  </div>

  <NewLandmarkPalette v-model:open="newLandmarkOpen" />
</template>
