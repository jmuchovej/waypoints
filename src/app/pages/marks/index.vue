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

  <div class="flex-1 overflow-auto">
    <div v-if="loading" class="flex h-full items-center justify-center">
      <UIcon name="i-lucide-loader" class="text-dimmed size-5 animate-spin" />
    </div>

    <div
      v-else-if="landmarks.length === 0"
      class="text-dimmed flex h-full flex-col items-center justify-center gap-2">
      <UIcon name="i-lucide-map-pin-off" class="size-8" />
      <p>No landmarks yet</p>
    </div>

    <nav v-else class="flex flex-col gap-0.5 p-2">
      <div
        v-for="mark in landmarks"
        :key="mark.id"
        role="link"
        tabindex="0"
        class="hover:bg-elevated focus:outline-primary flex cursor-pointer items-center gap-2 rounded-md py-1 ps-3 pe-1 transition-colors focus:outline focus:outline-2"
        @click="navigateTo(`/marks/${mark.id}`)"
        @keydown.enter="navigateTo(`/marks/${mark.id}`)">
        <span class="flex shrink-0 items-center" @click.stop>
          <UIcon :name="landmarkIcon(mark)" class="text-dimmed size-5" />
        </span>
        <RichTextEditor
          :rich-text="landmarkTitle(mark)!"
          :on-update="(rt: RichText) => onTitleUpdate(mark, rt)"
          placeholder="Untitled"
          class="min-w-0 shrink truncate"
          @click.stop />
        <UButton
          :to="`/marks/${mark.id}`"
          leading-icon="i-lucide-fullscreen"
          label="Open"
          variant="soft"
          color="neutral"
          size="sm"
          class="ms-auto shrink-0"
          @click.stop />
      </div>
    </nav>
  </div>

  <NewLandmarkPalette v-model:open="newLandmarkOpen" />
</template>
