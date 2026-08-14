<script setup lang="ts">
  import { isLandmark, isContent, type Mark } from "@waypoints/domain";

  import { LazyRichTextReadonly } from "#components";

  /**
   * Hydrated mention data from the backend.
   *
   * `mark` is absent when the referenced mark no longer exists in the database
   * (i.e., hard-deleted). In that case only the UUID remains — the mention
   * renders as a broken link.
   *
   * TODO: Distinguish soft-deleted (trashed) marks from hard-deleted — show
   *       muted/struck-through styling instead of a broken link.
   * TODO: Distinguish archived marks — separate visual treatment.
   * TODO: Consider preventing hard deletion when inbound mention references
   *       exist, or at least surfacing a warning.
   */
  type MentionData = { id: string; mark?: Mark };

  const props = defineProps<{
    data: MentionData;
  }>();

  const id = computed(() => props.data.id);
  const mentionMark = computed(() => props.data.mark ?? null);
  const isBroken = computed(() => !mentionMark.value);

  const mentionTitle = computed(() => {
    const m = mentionMark.value;
    if (!m) return null;
    if (isLandmark(m.data)) return m.data.title;
    return null;
  });

  const mentionIcon = computed(() => {
    const m = mentionMark.value;
    if (!m) return null;
    if (isLandmark(m.data)) return m.data.icon;
    return null;
  });

  const mentionContentType = computed(() => {
    const m = mentionMark.value;
    if (!m) return null;
    if (isContent(m.data)) return m.data.type;
    return null;
  });

  function handleClick() {
    if (id.value) {
      navigateTo(`/marks/${id.value}`);
    }
  }
</script>

<template>
  <UPopover mode="hover" :open-delay="300">
    <span
      data-inline="mention"
      :class="{ 'mention--broken': isBroken }"
      :data-id="id"
      @mousedown.stop
      @click.stop="handleClick">
      <slot />
    </span>

    <template #content>
      <div class="max-w-60 min-w-40 p-2">
        <template v-if="mentionTitle">
          <div class="flex items-center gap-2">
            <UIcon
              v-if="mentionIcon"
              :name="mentionIcon.identifier"
              class="text-dimmed size-4 shrink-0" />
            <span class="truncate text-sm font-medium">
              <LazyRichTextReadonly :rich-text="mentionTitle" />
            </span>
          </div>
        </template>
        <template v-else-if="mentionContentType">
          <span class="text-dimmed text-sm">{{ mentionContentType }}</span>
        </template>
        <template v-else>
          <span class="text-dimmed text-sm">Unknown mark</span>
        </template>
      </div>
    </template>
  </UPopover>
</template>
