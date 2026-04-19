<script setup lang="ts">
import type { Mark, Landmark, FeatureID, Reading, RichText, Chart } from "@waypoints/domain";
import DocumentBody from "~editor/components/DocumentBody.vue";
import RichTextEditor from "~editor/components/RichTextEditor.vue";
import ReadingsTable from "~editor/components/ReadingsTable.vue";
import { commands } from "~/bindings.g";
import { getCurrentWindow } from "@tauri-apps/api/window";

const log = createLogger("marks");

const route = useRoute();
const markID = route.params.markID as string;

const loading = ref(true);
const error = ref<string | null>(null);
const rawMark = ref<Mark | null>(null);
const showDebug = ref(false);

const landmark = computed<Landmark | null>(() => {
  const data = rawMark.value?.data;
  if (!data || data.mark !== "landmark") return null;
  return data;
});

const titleRichText = computed<RichText>(() => {
  return landmark.value?.title ?? { segments: [] };
});

const titlePlaceholder = computed(() => {
  const chartId = landmark.value?.chart;
  if (!chartId) return "Untitled";
  // ChartID is like "builtin.page" — extract the display name and capitalize
  const name = chartId.replace(/^builtin\./, "");
  return `Untitled ${name.charAt(0).toUpperCase()}${name.slice(1)}`;
});

const chart = ref<Chart | null>(null);

async function loadChart(chartId: string) {
  const result = await commands.chartGet(chartId);
  if (result.status === "ok") {
    chart.value = result.data;
  } else {
    log.error("chart_get failed:", result.error);
    chart.value = null;
  }
}

watch(
  () => landmark.value?.chart,
  (chartId) => {
    if (chartId) loadChart(chartId);
    else chart.value = null;
  },
  { immediate: true },
);

async function onReadingUpdate(featureId: FeatureID, reading: Reading) {
  log.info("onReadingUpdate", { featureId, reading });
  const result = await commands.editorUpdateMarkReading(markID, featureId, reading);
  if (result.status === "ok") {
    log.info("onReadingUpdate → rawMark updated", result.data);
    rawMark.value = result.data;
  } else {
    log.error("editor_update_mark_reading failed:", result.error);
  }
}

async function load() {
  loading.value = true;
  error.value = null;

  try {
    const result = await commands.editorMarkList();
    if (result.status !== "ok") {
      error.value = result.error;
      return;
    }

    const mark = result.data.find((m) => m.id === markID);
    if (!mark) {
      error.value = `Mark "${markID}" not found`;
      return;
    }

    rawMark.value = mark;
    await getCurrentWindow().setTitle(mark.value.plain_text);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);

function onTitleUpdate(richText: RichText) {
  commands.editorUpdateMarkTitle(markID, richText).then((res) => {
    if (res.status === "error") {
      log.error("editor_update_mark_title failed:", res.error);
    }
  });
}

const docBodyRef = ref<InstanceType<typeof DocumentBody> | null>(null);

/** Title → DocumentBody: Enter, ArrowDown, ArrowRight at end */
function onTitleKeyDown(e: KeyboardEvent): boolean {
  if (e.key === "Enter" || e.key === "ArrowDown") {
    e.preventDefault();
    focusFirstBlockOrCreate();
    return true;
  }
  return false;
}

async function focusFirstBlockOrCreate() {
  const doc = docBodyRef.value;
  if (!doc) return;
  const blocks = doc.blocks;
  if (blocks.length > 0) {
    doc.orchestrator.focus.focusBlock(blocks[0]!.id, "start");
  } else {
    await doc.orchestrator.onCreateFirstBlock();
  }
}

/** DocumentBody → Title: ArrowLeft at start, ArrowUp on first line */
function onDocExitTop() {
  // Focus the title editor's ProseMirror at the end
  const titleEl = document.querySelector(".landmark-title .ProseMirror") as HTMLElement | null;
  if (titleEl) {
    titleEl.focus();
    // Move cursor to end
    const sel = window.getSelection();
    if (sel) {
      sel.selectAllChildren(titleEl);
      sel.collapseToEnd();
    }
  }
}

const uiPanel = {
  body: "p-0 sm:p-0",
};
</script>

<template>
  <AppHeader :tabs="[]" :title="titleRichText" />

  <div class="flex-1 pt-8 overflow-y-auto editor-layout">
    <div class="editor-content landmark-title text-3xl font-bold">
      <USkeleton v-if="loading" class="min-h-8 w-full" />
      <RichTextEditor
        v-else-if="landmark"
        :rich-text="titleRichText"
        :on-update="onTitleUpdate"
        :placeholder="titlePlaceholder"
        :on-key-down="onTitleKeyDown"
      />
    </div>

    <div v-if="landmark && chart" class="editor-content">
      <ReadingsTable :chart="chart" :readings="landmark.readings" :on-update="onReadingUpdate" />
    </div>

    <USeparator class="editor-content border-default my-4" />

    <div class="editor-content">
      <DocumentBody ref="docBodyRef" :landmark-id="markID" :on-exit-top="onDocExitTop" />
    </div>
  </div>
</template>
