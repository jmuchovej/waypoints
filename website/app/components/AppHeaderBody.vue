<script setup lang="ts">
  import type { ContentNavigationItem } from "@nuxt/content";

  import { resourceLinks } from "~/data/navigation";

  const navigation = inject<Ref<ContentNavigationItem[]>>("navigation");
  const contentNavVariants = useUIConfig("contentNavigation");
</script>

<template>
  <UNavigationMenu :items="resourceLinks" orientation="vertical" variant="link" />

  <USeparator class="py-4" />

  <UContentNavigation
    :highlight="contentNavVariants.highlight ?? true"
    :highlight-color="contentNavVariants.highlightColor"
    :variant="contentNavVariants.variant ?? 'link'"
    :color="contentNavVariants.color"
    :navigation="navigation" />
</template>
