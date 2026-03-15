<script setup lang="ts">
import { commands } from "~/bindings.g";
import type { Mark, RichText, Chart, ChartID } from "@waypoints/domain";
import { isLandmark } from "@waypoints/domain";
import RichTextEditor from "~editor/components/RichTextEditor.vue";
import { iconName } from "~/utils/icon";

const log = createLogger("marks");

const landmarks = ref<Mark[]>([]);
const loading = ref(true);
/** Fallback icon when neither the landmark nor its chart has one set. */
const FALLBACK_ICON = "i-lucide-file-text";

async function load() {
  loading.value = true;

  // Fetch marks and charts in parallel — the chart lookup feeds the icon
  // resolution path so we need both before rendering.
  const [markResult, chartResult] = await Promise.all([
    commands.markList(),
    commands.chartList(),
  ]);

  if (markResult.status === "ok") {
    landmarks.value = markResult.data;
  } else {
    log.warn("mark_list failed:", markResult.error);
  }

  if (chartResult.status === "ok") {
    charts.value = new Map(chartResult.data.map((c) => [c.id, c]));
  } else {
    log.warn("chart_list failed:", chartResult.error);
  }

  loading.value = false;
}

onMounted(load);

function landmarkTitle(mark: Mark): RichText | null {
  if (isLandmark(mark.data)) return mark.data.title;
  return null;
}

/**
 * Resolve the icon for a landmark list item. Priority:
 *   1. The landmark's own `icon` (user-chosen / instance-specific)
 *   2. The landmark's chart `icon` (schema default)
 *   3. A generic fallback
 *
 * This guarantees the list item always has an icon — every landmark is
 * backed by a chart, and every builtin chart has an icon.
 */
function landmarkIcon(mark: Mark): string {
  if (!isLandmark(mark.data)) return FALLBACK_ICON;

  // 1. Landmark's own icon
  const own = iconName(mark.data.icon);
  if (own) return own;

  // 2. Chart's icon
  const chart = charts.value.get(mark.data.chart);
  const chartIcon = iconName(chart?.icon);
  if (chartIcon) return chartIcon;

  // 3. Fallback
  return FALLBACK_ICON;
}

async function onTitleUpdate(mark: Mark, richText: RichText) {
  if (!isLandmark(mark.data)) return;
  mark.data.title = richText;
  const result = await commands.editorUpdateMarkTitle(mark.id, richText);
  if (result.status === "error") {
    log.warn("update title failed:", result.error);
  }
}
</script>

<template>
  <UDashboardPanel id="marks">
    <template #header>
      <UDashboardNavbar title="Marks">
        <template #leading>
          <UDashboardSidebarCollapse />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
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
    </template>
  </UDashboardPanel>
</template>
