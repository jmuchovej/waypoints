<script setup lang="ts">
import { commands } from "~/bindings.g";
import type { Mark } from "@waypoints/domain";

const log = createLogger("marks");

const landmarks = ref<Mark[]>([]);
const loading = ref(true);

async function load() {
  loading.value = true;
  const result = await commands.markList();
  log.info(result);
  if (result.status === "ok") {
    landmarks.value = result.data;
  } else {
    log.warn("mark_list failed:", result.error);
  }
  loading.value = false;
}

onMounted(load);

function landmarkTitle(mark: Mark): string {
  if (mark.data.mark === "landmark") {
    return mark.data.title.plain_text || "Untitled";
  }
  return "Untitled";
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
        <NuxtLink
          v-for="mark in landmarks"
          :key="mark.id"
          :to="`/marks/${mark.id}`"
          class="flex items-center gap-2 px-3 py-2 rounded-md hover:bg-elevated transition-colors"
        >
          <UIcon
            name="i-lucide-file-text"
            class="size-4 text-dimmed shrink-0"
          />
          <span class="truncate">{{ landmarkTitle(mark) }}</span>
        </NuxtLink>
      </nav>
    </template>
  </UDashboardPanel>
</template>
