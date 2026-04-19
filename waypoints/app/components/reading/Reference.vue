<script lang="ts">
import type { Reading, Feature, FeatureData } from "@waypoints/domain";

export interface ReferenceReadingProps {
  value: Reading | undefined;
  feature: Feature;
}

export interface ReferenceReadingEmits {
  update: [reading: Reading];
}
</script>
<script setup lang="ts">
import type { MarkID, LandmarkMark } from "@waypoints/domain";
import { ipc } from "#imports";

const props = defineProps<ReferenceReadingProps>();
const emit = defineEmits<ReferenceReadingEmits>();

const refKind = computed(() => props.feature.kind as FeatureData<"reference">["kind"]);

interface RefItem {
  id: MarkID;
  label: string;
  icon: string;
}

const allLandmarks = ref<LandmarkMark[]>([]);
const loadError = ref<string | null>(null);
const loading = ref(true);

async function loadMarks() {
  loading.value = true;
  loadError.value = null;
  try {
    const allowedTypes = refKind.value.allowed_types ?? undefined;
    allLandmarks.value = await ipc.marks.list("landmark", { allowedTypes });
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : "Failed to load landmarks";
  } finally {
    loading.value = false;
  }
}

onMounted(loadMarks);

const items = computed<RefItem[]>(() =>
  allLandmarks.value.map((m) => ({
    id: m.id,
    label:
      m.data.title.plain_text || m.data.title.segments.map((s) => s.text).join("") || "Untitled",
    icon: "i-lucide-file-text",
  })),
);

const selectedIds = computed<MarkID[]>(() => {
  if (props.value?.type === "references") return props.value.value;
  return [];
});

function onUpdate(val: MarkID[] | null) {
  if (!val || val.length === 0) {
    emit("update", { type: "null" });
    return;
  }
  emit("update", { type: "references", value: val });
}
</script>

<template>
  <UInputMenu
    :model-value="selectedIds"
    :items="items"
    multiple
    size="md"
    variant="ghost"
    value-key="id"
    label-key="label"
    delete-icon="i-lucide-x"
    trailing-icon="i-lucide-plus"
    clear-icon="i-lucide-trash"
    open-on-click
    @update:model-value="onUpdate"
  >
    <template #empty>
      <p v-if="loading" class="text-sm text-dimmed p-2">Loading…</p>
      <p v-else-if="loadError" class="text-sm text-error p-2">
        {{ loadError }}
      </p>
      <p v-else class="text-sm text-dimmed p-2">No landmarks found</p>
    </template>
  </UInputMenu>
</template>
