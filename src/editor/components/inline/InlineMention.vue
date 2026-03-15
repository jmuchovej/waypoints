<script setup lang="ts">
import { commands } from "~/bindings.g";
import type { Mark, MarkID } from "@waypoints/domain";

const props = defineProps<{
  data: unknown;
}>();

const id = (props.data as Record<string, unknown> | null)?.id as string | undefined;

function handleClick() {
  if (id) {
    navigateTo(`/marks/${id}`);
  }
}

// ── Hover card data ──────────────────────────────────────────────────────

const mentionCache = new Map<string, Mark>();
const mentionData = ref<Mark | null>(null);
const loading = ref(false);

async function fetchMention() {
  if (!id || mentionData.value) return;

  const cached = mentionCache.get(id);
  if (cached) {
    mentionData.value = cached;
    return;
  }

  loading.value = true;
  try {
    const result = await commands.editorGetMark(id as MarkID);
    if (result.status === "ok") {
      mentionData.value = result.data;
      mentionCache.set(id, result.data);
    }
  } catch {
    // Silently fail if backend unavailable
  } finally {
    loading.value = false;
  }
}

const mentionLabel = computed(() => {
  if (!mentionData.value) return undefined;
  const d = mentionData.value.data;
  if ("mark" in d) {
    if (d.mark === "landmark" && "title" in d) {
      const title = d.title as { plain_text?: string } | undefined;
      return title?.plain_text ?? "Untitled";
    }
    if (d.mark === "content" && "type" in d) {
      return String(d.type);
    }
  }
  return "Mark";
});

const mentionIcon = computed(() => {
  if (!mentionData.value) return "i-lucide-file";
  const d = mentionData.value.data;
  if ("mark" in d && d.mark === "landmark") return "i-lucide-landmark";
  return "i-lucide-file-text";
});
</script>

<template>
  <UPopover mode="hover" :open-delay="300" @open="fetchMention">
    <span data-inline="mention" class="mention" :data-id="id" @click.stop="handleClick">
      <slot />
    </span>

    <template #content>
      <div class="p-2 min-w-40 max-w-60">
        <div v-if="loading" class="flex items-center gap-2 text-sm text-dimmed">
          <UIcon name="i-lucide-loader-2" class="size-4 animate-spin" />
          Loading...
        </div>
        <div v-else-if="mentionData" class="flex items-center gap-2">
          <UIcon :name="mentionIcon" class="size-4 shrink-0 text-dimmed" />
          <span class="text-sm font-medium truncate">{{ mentionLabel }}</span>
        </div>
        <div v-else class="text-sm text-dimmed">Unknown mark</div>
      </div>
    </template>
  </UPopover>
</template>
