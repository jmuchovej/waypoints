<script setup lang="ts">
import {
  BlockOrchestratorKey,
  CommandMenuKey,
  type OrchestratorContext,
  type CommandMenuContext,
  filterSlashItems,
  groupSlashItems,
  type SlashMenuItem,
} from "#imports";
import type { RichText } from "@waypoints/domain";

const overlay = inject<CommandMenuContext>(CommandMenuKey);
const orchestrator = inject<OrchestratorContext>(BlockOrchestratorKey);

const selectedIndex = ref(0);

const filteredItems = computed(() => {
  if (!overlay?.activeMenu.value) return [];
  return filterSlashItems(overlay.activeMenu.value.query);
});

const groupedItems = computed(() => groupSlashItems(filteredItems.value));

watch(
  () => overlay?.activeMenu.value?.query,
  () => {
    selectedIndex.value = 0;
  },
);

function selectItem(item: SlashMenuItem) {
  if (!overlay?.activeMenu.value || !orchestrator) return;
  const blockId = overlay.activeMenu.value.blockID;
  const block = orchestrator.blocks.value.find((b) => b.id === blockId);

  // Extract current rich_text, stripping the /query trigger text
  let currentRichText: RichText | undefined;
  if (block?.data.mark === "content" && "rich_text" in block.data) {
    const rt = block.data.rich_text as RichText;
    // Strip the slash command from the text (e.g., "/heading1" or "/ heading1")
    const cleaned = stripSlashTrigger(rt);
    currentRichText = cleaned.segments.length > 0 ? cleaned : undefined;
  }

  const newContent = item.toContent(currentRichText);
  orchestrator.onChangeBlockType(blockId, newContent);
  overlay.hide();
}

/** Remove the /query text from rich_text segments. */
function stripSlashTrigger(rt: RichText): RichText {
  // The slash trigger is always at the end of the text: ".../<query>"
  // We need to find and remove the last occurrence of /\S* pattern
  const segments = rt.segments.map((s) => ({ ...s }));
  if (segments.length === 0) return { plain_text: "", segments };

  // Find the last segment that contains the slash trigger
  for (let i = segments.length - 1; i >= 0; i--) {
    const seg = segments[i]!;
    const slashIdx = seg.text.lastIndexOf("/");
    if (slashIdx >= 0) {
      // Remove from the slash to the end of this segment
      seg.text = seg.text.slice(0, slashIdx).trimEnd();
      // Remove any trailing empty segments
      if (seg.text === "") {
        segments.splice(i, 1);
      }
      // Remove all segments after this one (they were part of the query)
      segments.splice(i + 1);
      break;
    }
  }

  return { plain_text: segments.map((s) => s.text).join(""), segments };
}

function handleKeydown(e: KeyboardEvent) {
  if (!overlay?.isOpen.value) return;

  const items = filteredItems.value;
  if (items.length === 0) return;

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
      v-if="overlay?.isOpen.value && overlay.activeMenu.value?.kind === 'slash'"
      class="fixed z-50 bg-elevated border border-default rounded-lg shadow-lg py-1 w-64 max-h-80 overflow-y-auto"
      :style="{
        left: `${overlay.activeMenu.value.position.x}px`,
        top: `${overlay.activeMenu.value.position.y}px`,
      }"
    >
      <div v-if="filteredItems.length === 0" class="px-3 py-2 text-sm text-dimmed italic">
        No results
      </div>
      <template v-for="group in groupedItems" :key="group.category">
        <div
          class="px-3 py-1 text-xs font-semibold text-dimmed uppercase tracking-wider first:pt-1"
        >
          {{ group.label }}
        </div>
        <button
          v-for="item in group.items"
          :key="item.id"
          class="w-full flex items-center gap-3 px-3 py-1.5 text-sm text-default cursor-pointer transition-colors"
          :class="{
            'bg-accented': filteredItems.indexOf(item) === selectedIndex,
          }"
          @click="selectItem(item)"
          @mouseenter="selectedIndex = filteredItems.indexOf(item)"
        >
          <UIcon :name="item.icon" class="size-4 shrink-0" />
          <div class="flex flex-col flex-1 text-start">
            <span class="font-medium">{{ item.label }}</span>
            <span class="text-xs text-dimmed">{{ item.description }}</span>
          </div>
        </button>
      </template>
    </div>
  </Teleport>
</template>
