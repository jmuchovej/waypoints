<script lang="ts">
import type { Reading, Feature } from "@waypoints/domain";

export interface ReferenceReadingProps {
  value: Reading | undefined;
  feature: Feature;
}

export interface ReferenceReadingEmits {
  update: [reading: Reading];
}
</script>
<script setup lang="ts">
import { type MarkID, type Mark, isLandmark } from "@waypoints/domain";
import { commands } from "~/bindings.g";

const props = defineProps<ReferenceReadingProps>();
const emit = defineEmits<ReferenceReadingEmits>();

interface RefItem {
  id: MarkID;
  label: string;
  icon: string;
}

const allMarks = ref<Mark[]>([]);

async function loadMarks() {
  const result = await commands.markList();
  if (result.status === "ok") {
    allMarks.value = result.data;
  }
}

onMounted(loadMarks);

// Filter by allowed_types from the feature kind
const items = computed<RefItem[]>(() => {
  const k = props.feature.kind;
  const allowedTypes = k.type === "reference" ? k.allowed_types : null;

  return allMarks.value
    .filter((m) => {
      if (!isLandmark(m)) return false;
      if (!allowedTypes || allowedTypes.length === 0) return true;
      return allowedTypes.includes(m.data.chart);
    })
    .map((m) => {
      const title = isLandmark(m)
        ? (m.data.title.plain_text ?? m.data.title.segments.map((s) => s.text).join(""))
        : "";
      return {
        id: m.id,
        label: title || "Untitled",
        icon: "i-lucide-file-text",
      };
    });
});

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
    variant="ghost"
    value-key="id"
    label-key="label"
    placeholder="Pick references…"
    delete-icon="i-lucide-trash"
    clear-icon="i-lucide-trash"
    open-on-click
    @update:model-value="onUpdate"
  />
</template>
