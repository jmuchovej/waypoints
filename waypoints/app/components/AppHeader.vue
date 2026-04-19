<script lang="ts">
import type { NavigationMenuItem } from "@nuxt/ui";
import type { RichText } from "@waypoints/domain";
import { RichTextReadonly } from "#components";
import { createLogger } from "#imports";

export interface AppHeaderProps {
  title: RichText;
  tabs: NavigationMenuItem[];
}
</script>

<script setup lang="ts">
import type { Ref } from "vue";
const logger = createLogger("AppHeader");

const open = inject<Ref<boolean>>("sidebar-open", ref(false));
const newLandmarkOpen = inject<Ref<boolean>>("new-landmark-open", ref(false));
const props = defineProps<AppHeaderProps>();
logger.debug(props.title);
</script>

<template>
  <UHeader
    :ui="{
      left: 'min-w-0',
      container: 'max-w-none px-4 sm:px-4 lg:px-4',
    }"
    class="flex flex-col"
    :class="{ toolbar: tabs.length > 0 }"
  >
    <template #left>
      <UButton
        icon="i-lucide-panel-left"
        size="sm"
        variant="ghost"
        color="neutral"
        @click="open = !open"
      />

      <RichTextReadonly :rich-text="title" class="text-lg font-semibold truncate" />
    </template>

    <template #right>
      <UTooltip text="New landmark" :kbds="['meta', 'N']">
        <UButton
          icon="i-lucide-plus"
          label="New"
          size="sm"
          color="primary"
          @click="newLandmarkOpen = true"
        />
      </UTooltip>
    </template>

    <template v-if="tabs.length > 0" #bottom>
      <USeparator class="hidden lg:flex" />

      <UContainer class="hidden lg:flex items-center justify-between">
        <UNavigationMenu :items="tabs" highlight variant="pill" class="-mx-2.5 -mb-px" />
      </UContainer>
    </template>
  </UHeader>
</template>

<style>
@reference "~/assets/css/main.css";

@media (min-width: 1024px) {
  .toolbar {
    --ui-header-height: --spacing(28);
  }
}
</style>
